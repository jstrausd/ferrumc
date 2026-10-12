//! Disconnect (D2), detach confirmation (D3), the leave-save service (D4,
//! §7.3), retirement (D5, .2c amendment R1), lifecycle acknowledgement (D7/D8,
//! .2c amendment R2), release (§7.5) and the G3 adoption (§8) of native
//! sessions (NET-04.2e2).
//!
//! The session is the single producer of the leave-save autosave, of the G3
//! disconnect and of the release call (B2); the driver only holds the opaque
//! [`NativeRetentionToken`] and lends its shard to closures.

use std::sync::Arc;

use ferrumc_core::PlayerId;
use ferrumc_sim::{
    ConnectedOwnedPlayerView, DetachedOwnedPlayerStatus, OwnedPlayerHandle, OwnedPlayerInput,
    OwnershipTransferId, PlayerOwnershipError, ReleasedOwnedPlayer,
};

use super::{
    DisconnectState, EntryState, NativePhase, RecordPhase, RetainedRecord, SessionRouter,
    Suspension,
};
use crate::native::{
    AdoptionInvariant, AdoptionInvariantKind, CollectorSessionError, Decision, DisconnectPath,
    LaneDisconnect, LaneMark, LeaveSaveQuarantine, NativeAcknowledgeReport, NativeDetachOutcome,
    NativeInputClass, NativeJoinReservation, NativeLifecycleProbe, NativeLifecycleVerdict,
    NativeRetentionPhase, NativeRetentionStatus, NativeRetentionStep, NativeRetentionToken,
    NativeRetirementReport, NativeShardInput, PlayerSessionKey, Retention,
};
use crate::translate::{player_info_remove, remove_entities_shell};
use crate::OutboundMessage;

/// The driver's lifecycle closure: stored handle plus single-use probe in,
/// sealed verdict out.
type LifecycleClosure<'c> =
    dyn FnMut(&OwnedPlayerHandle, NativeLifecycleProbe<'_>) -> NativeLifecycleVerdict + 'c;

impl SessionRouter {
    /// D2: starts the native leave of the session named by `key` and returns
    /// its retention token (A5).
    ///
    /// The session becomes `Leaving` in its own table entry (no allocation):
    /// its outbound lane and movement cell close, its viewer baselines are
    /// dropped, and exactly one `OwnedPlayerInput::disconnect` built from the
    /// stored handle is queued as a control item (it may use the reserved
    /// tail). A full or closed lane, or a suspended endpoint, retains the
    /// disconnect deduplicated; [`retry_native_disconnects`](Self::retry_native_disconnects)
    /// or revalidation sends it first. The first call assigns the lineage from
    /// the router's checked counter; repeated calls are idempotent and return
    /// the same token.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::StaleSession`] for a foreign, stale, retired or
    /// unknown key, or [`CollectorSessionError::LineageExhausted`]; nothing
    /// changes then.
    pub fn disconnect_native(
        &mut self,
        key: &PlayerSessionKey,
    ) -> Result<NativeRetentionToken, CollectorSessionError> {
        let (slot, session) = self
            .native_session(key)
            .ok_or(CollectorSessionError::StaleSession)?;
        let lineage = match &session.retention {
            Some(retention) => retention.lineage,
            None => self.native.take_lineage()?,
        };
        if let Some(session) = self.native.session_mut(slot) {
            if session.retention.is_none() {
                session.retention = Some(Retention::new(lineage));
            }
        }
        self.begin_native_leave(slot);
        Ok(NativeRetentionToken {
            player: key.player,
            lineage,
        })
    }

    /// Read-only diagnostics of the retention record named by `token`.
    pub fn native_retention_status(
        &self,
        token: NativeRetentionToken,
    ) -> Option<NativeRetentionStatus> {
        let slot = self.retention_slot(token)?;
        match &self.native.entry(slot)?.state {
            EntryState::Session(session) => {
                let retention = session.retention.as_ref()?;
                Some(NativeRetentionStatus {
                    phase: if session.detached {
                        NativeRetentionPhase::Detached
                    } else {
                        NativeRetentionPhase::Leaving
                    },
                    leave_save: retention.state(),
                    attempts: retention.attempts,
                    bound: true,
                })
            }
            EntryState::Record(record) => Some(NativeRetentionStatus {
                phase: match record.phase {
                    RecordPhase::Leaving => NativeRetentionPhase::Leaving,
                    RecordPhase::Detached => NativeRetentionPhase::Detached,
                    RecordPhase::Retired { acked } => NativeRetentionPhase::Retired {
                        acknowledged: acked,
                    },
                    RecordPhase::Superseded { acked } => NativeRetentionPhase::Superseded {
                        acknowledged: acked,
                    },
                },
                leave_save: record.retention.state(),
                attempts: record.retention.attempts,
                bound: record.bound,
            }),
            _ => None,
        }
    }

    /// D3: confirms the genuine detach fact of `player`'s leaving native entry.
    ///
    /// `PlayerDetached{player}` is only a trigger. The closure receives the
    /// stored original handle and a single-use probe and submits the genuine
    /// `collector_lifecycle_view` of its shard borrow; the session requires
    /// that the view and its detach fact name exactly the stored owner,
    /// admission and the registration's full generation. Afterwards a control
    /// lane input of the session is no longer live (A4) and no lane disconnect
    /// counts as retained.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::ProbesExhausted`]; nothing changes then.
    pub fn native_detached(
        &mut self,
        player: PlayerId,
        probe: &mut LifecycleClosure<'_>,
    ) -> Result<NativeDetachOutcome, CollectorSessionError> {
        let found = self.native.entries().find_map(|(slot, entry)| {
            if entry.player != player {
                return None;
            }
            match &entry.state {
                EntryState::Session(s) if s.phase == NativePhase::Leaving && !s.detached => {
                    Some((slot, s.handle.clone(), s.binding.generation))
                }
                EntryState::Record(r) if r.phase == RecordPhase::Leaving => {
                    Some((slot, r.original.clone(), r.binding.generation))
                }
                _ => None,
            }
        });
        let Some((slot, handle, generation)) = found else {
            return Ok(NativeDetachOutcome::NoRecord);
        };
        let seq = self.next_probe()?;
        let verdict = probe(
            &handle,
            NativeLifecycleProbe {
                expected: &handle,
                generation,
                seq,
                ack: false,
            },
        );
        if !(verdict.seq == seq && verdict.facts.is_some_and(|facts| facts.detached)) {
            return Ok(NativeDetachOutcome::NotDetached);
        }
        match self.native.entry_mut(slot).map(|entry| &mut entry.state) {
            Some(EntryState::Session(session)) => {
                session.detached = true;
                if session.disconnect == DisconnectState::Retained {
                    session.disconnect = DisconnectState::NotNeeded;
                }
            }
            Some(EntryState::Record(record)) => record.phase = RecordPhase::Detached,
            _ => {}
        }
        Ok(NativeDetachOutcome::Confirmed)
    }

    /// A5: a `PlayerChanged`/`Resync` intent whose handle has no connected
    /// view. The session compares the transient handle (owner, player,
    /// admission) with its retention records and returns the matching token,
    /// so the driver services that record immediately (§7.3 point 6). `None`
    /// means a stale intent (metric `native_stale_intent`, discarded).
    pub fn native_detached_intent(
        &self,
        handle: &OwnedPlayerHandle,
    ) -> Option<NativeRetentionToken> {
        self.native
            .entries()
            .find_map(|(_, entry)| match &entry.state {
                EntryState::Session(session) if session.handle == *handle => session
                    .retention
                    .as_ref()
                    .map(|retention| NativeRetentionToken {
                        player: entry.player,
                        lineage: retention.lineage,
                    }),
                EntryState::Record(record) if record.original == *handle => {
                    Some(NativeRetentionToken {
                        player: entry.player,
                        lineage: record.retention.lineage,
                    })
                }
                _ => None,
            })
    }

    /// D4: one bounded step of the leave-save state machine of the record
    /// named by `token` (§7.3 including R2-A1).
    ///
    /// `applied` is the driver's count of successfully applied normal
    /// boundaries; `submit_allowed` says whether the gate admits one more gated
    /// class-2 submit in this iteration. `status` must return
    /// `detached_owned_player_status` of the stored original handle; `ids`
    /// supplies a fresh driver operation id when a leave-save is submitted.
    /// The disconnect of a mapped session is resent over its lane (at most
    /// [`MAX_DISCONNECT_RESENDS`](crate::MAX_DISCONNECT_RESENDS) times), the
    /// disconnect of an adopted admission is a [`NativeRetentionStep::Submit`].
    /// §7.3 amendment V1 (Root): a resend counts only after the previous lane
    /// disconnect was taken from its lane and a normal boundary followed; a
    /// disconnect still queued behind other items never counts.
    /// A stale or foreign handle quarantines the record typed; it is never
    /// resent and never rebound.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::StaleRetention`] for an unknown token, or
    /// [`CollectorSessionError::NativeStatusInvariant`] when the status closure
    /// reports a wiring break (snapshot, uninitialized, scope); the driver
    /// fail-stops and the record is unchanged.
    pub fn service_native_retention(
        &mut self,
        token: NativeRetentionToken,
        applied: u64,
        submit_allowed: bool,
        status: &mut dyn FnMut(
            &OwnedPlayerHandle,
        ) -> Result<DetachedOwnedPlayerStatus, PlayerOwnershipError>,
        ids: &mut dyn FnMut() -> OwnershipTransferId,
    ) -> Result<NativeRetentionStep, CollectorSessionError> {
        let slot = self
            .retention_slot(token)
            .ok_or(CollectorSessionError::StaleRetention)?;
        let (handle, key, path) = match self.native.entry(slot).map(|entry| &entry.state) {
            Some(EntryState::Session(session)) => {
                let path = match (session.suspension, session.disconnect) {
                    // The simulation already reported the player detached at
                    // revalidation: should it still be connected, only a
                    // gated submit can reach it.
                    (Suspension::Detached, _) => DisconnectPath::Submit,
                    (_, DisconnectState::NotNeeded) => DisconnectPath::Lane(LaneDisconnect::Taken),
                    // §7.3 amendment V1 (Root): only a disconnect the receiver
                    // took out of its lane can count a resend.
                    (_, DisconnectState::Sent) => DisconnectPath::Lane(
                        if session
                            .disconnect_mark
                            .as_ref()
                            .is_some_and(LaneMark::taken)
                        {
                            LaneDisconnect::Taken
                        } else {
                            LaneDisconnect::Queued
                        },
                    ),
                    _ => DisconnectPath::Lane(LaneDisconnect::Retained),
                };
                (session.handle.clone(), session.key.clone(), path)
            }
            Some(EntryState::Record(record)) => {
                if matches!(record.phase, RecordPhase::Superseded { .. }) {
                    return Ok(NativeRetentionStep::Superseded);
                }
                (
                    record.original.clone(),
                    record.key.clone(),
                    DisconnectPath::Submit,
                )
            }
            _ => return Err(CollectorSessionError::StaleRetention),
        };
        let observed = status(&handle);
        let decision = match self.retention_mut(slot) {
            Some(retention) => retention.step(applied, submit_allowed, path, &observed, ids),
            None => return Err(CollectorSessionError::StaleRetention),
        };
        Ok(match decision {
            Decision::SubmitDisconnect => NativeRetentionStep::Submit(NativeShardInput {
                key,
                class: NativeInputClass::Control,
                input: OwnedPlayerInput::disconnect(handle),
            }),
            Decision::SubmitAutosave(id) => NativeRetentionStep::Submit(NativeShardInput {
                key,
                class: NativeInputClass::Control,
                input: OwnedPlayerInput::autosave(handle, id),
            }),
            Decision::ResendLaneDisconnect => {
                if let Some(session) = self.native.session_mut(slot) {
                    session.disconnect = DisconnectState::Retained;
                }
                // A resend the lane cannot take stays retained; the router's
                // retry sends it, and only its take starts the next wait.
                self.flush_disconnect(slot);
                NativeRetentionStep::Waiting
            }
            Decision::Waiting => NativeRetentionStep::Waiting,
            Decision::Confirmed => NativeRetentionStep::SaveConfirmed,
            Decision::Quarantined(reason) => {
                let attempts = self.retention_mut(slot).map_or(0, |r| r.attempts);
                NativeRetentionStep::Quarantined(LeaveSaveQuarantine {
                    player: token.player,
                    reason,
                    attempts,
                })
            }
            Decision::StatusInvariant => {
                return Err(CollectorSessionError::NativeStatusInvariant);
            }
        })
    }

    /// The simulation refused the enqueue of the submit the last service step
    /// returned; the record goes back without a counted attempt.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::StaleRetention`] or
    /// [`CollectorSessionError::NoRetentionSubmit`] when the last step of this
    /// record returned no submit; nothing changes then.
    pub fn native_retention_unsent(
        &mut self,
        token: NativeRetentionToken,
    ) -> Result<(), CollectorSessionError> {
        let slot = self
            .retention_slot(token)
            .ok_or(CollectorSessionError::StaleRetention)?;
        let retention = self
            .retention_mut(slot)
            .ok_or(CollectorSessionError::StaleRetention)?;
        if retention.unsent() {
            Ok(())
        } else {
            Err(CollectorSessionError::NoRetentionSubmit)
        }
    }

    /// Stop phase A "final save" (§7.3, §10.4): renews exactly one leave-save
    /// attempt of a quarantined record, once per record. Returns `false` when
    /// the record is not quarantined or its final save was already used.
    ///
    /// A renewed `DisconnectNotApplied` record whose player is still connected
    /// is legitimate, no wiring break: its next service step returns it to
    /// that quarantine without an attempt or resend (F1, round 2). If its
    /// disconnect was applied meanwhile, the renewed attempt is a genuine
    /// leave-save.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::StaleRetention`].
    pub fn renew_native_leave_save(
        &mut self,
        token: NativeRetentionToken,
    ) -> Result<bool, CollectorSessionError> {
        let slot = self
            .retention_slot(token)
            .ok_or(CollectorSessionError::StaleRetention)?;
        Ok(self
            .retention_mut(slot)
            .ok_or(CollectorSessionError::StaleRetention)?
            .renew_final_save())
    }

    /// D5 (.2c amendment R1): retires every formerly active, leaving session
    /// whose genuine detach fact was confirmed and proven again by the view.
    ///
    /// For each candidate the closure receives the stored original handle and
    /// a single-use probe and submits the genuine `collector_lifecycle_view`.
    /// On a matching fact the key mapping is removed (old events of the key
    /// become `StaleSession`), every viewer holding a baseline of the subject
    /// gets a mandatory `EntityRemove` (when spawned) and then a mandatory
    /// `TabRemove`, and the entry becomes a `Retired` retention record that
    /// keeps the original handle until release. A viewer that cannot accept a
    /// mandatory remove is set leaving; the retirement is not blocked.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::AllocationFailed`] for the bounded worklist or
    /// [`CollectorSessionError::ProbesExhausted`]; nothing changes then.
    pub fn retire_detached_native_sessions(
        &mut self,
        probe: &mut LifecycleClosure<'_>,
    ) -> Result<NativeRetirementReport, CollectorSessionError> {
        let mut worklist = Vec::new();
        worklist
            .try_reserve_exact(self.native.slots.len())
            .map_err(|_| CollectorSessionError::AllocationFailed)?;
        worklist.extend(
            self.native
                .sessions()
                .filter(|(_, s)| {
                    s.ever_active
                        && s.phase == NativePhase::Leaving
                        && s.detached
                        && s.retention.is_some()
                })
                .map(|(slot, _)| slot),
        );
        self.reserve_probes(worklist.len())?;
        let mut report = NativeRetirementReport::default();
        for slot in worklist {
            let Some((handle, generation)) = self
                .native
                .session(slot)
                .map(|s| (s.handle.clone(), s.binding.generation))
            else {
                continue;
            };
            let seq = self.next_probe()?;
            let verdict = probe(
                &handle,
                NativeLifecycleProbe {
                    expected: &handle,
                    generation,
                    seq,
                    ack: false,
                },
            );
            if verdict.seq == seq && verdict.facts.is_some_and(|facts| facts.detached) {
                self.retire_session(slot);
                report.retired += 1;
            } else {
                report.waiting += 1;
            }
        }
        Ok(report)
    }

    /// D7/D8 (.2c amendment R2, Root R3-A1 form): acknowledges the lifecycle
    /// cell of every retired or superseded record whose original admission is
    /// detached with zero source pins.
    ///
    /// One closure per record: it submits the genuine view to the probe; the
    /// verdict holds the owned lifecycle ticket and lends it once to the
    /// driver's `acknowledge_collector_lifecycle` through
    /// [`NativeLifecycleVerdict::acknowledge`]. A retired record stays until
    /// release (`Retired{acknowledged}`); a superseded record is removed. The
    /// consumer references of NET-04.2b do not exist yet and count zero.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::NativeLifecycleAckFailed`] when the simulation
    /// refused an authorized acknowledgement (invariant break, driver
    /// fail-stop; later records are not processed),
    /// [`CollectorSessionError::AllocationFailed`] or
    /// [`CollectorSessionError::ProbesExhausted`].
    pub fn acknowledge_retired_native_records(
        &mut self,
        probe: &mut LifecycleClosure<'_>,
    ) -> Result<NativeAcknowledgeReport, CollectorSessionError> {
        let mut worklist = Vec::new();
        worklist
            .try_reserve_exact(self.native.slots.len())
            .map_err(|_| CollectorSessionError::AllocationFailed)?;
        worklist.extend(
            self.native
                .entries()
                .filter_map(|(slot, entry)| match &entry.state {
                    EntryState::Record(RetainedRecord {
                        phase:
                            RecordPhase::Retired { acked: false }
                            | RecordPhase::Superseded { acked: false },
                        ..
                    }) => Some(slot),
                    _ => None,
                }),
        );
        self.reserve_probes(worklist.len())?;
        let mut report = NativeAcknowledgeReport::default();
        for slot in worklist {
            let Some((handle, generation)) = self
                .record(slot)
                .map(|r| (r.original.clone(), r.binding.generation))
            else {
                continue;
            };
            let seq = self.next_probe()?;
            let verdict = probe(
                &handle,
                NativeLifecycleProbe {
                    expected: &handle,
                    generation,
                    seq,
                    ack: true,
                },
            );
            if verdict.seq != seq {
                report.waiting += 1;
                continue;
            }
            match verdict.acknowledged {
                Some(true) => {
                    let mut superseded = false;
                    if let Some(EntryState::Record(record)) =
                        self.native.entry_mut(slot).map(|e| &mut e.state)
                    {
                        if matches!(record.phase, RecordPhase::Superseded { .. }) {
                            superseded = true;
                        } else {
                            record.phase = RecordPhase::Retired { acked: true };
                        }
                    }
                    if superseded {
                        self.native.free(slot);
                        report.removed += 1;
                    } else {
                        report.acknowledged += 1;
                    }
                }
                Some(false) => return Err(CollectorSessionError::NativeLifecycleAckFailed),
                None => report.waiting += 1,
            }
        }
        Ok(report)
    }

    /// §7.5: releases the detached simulation player of the record named by
    /// `token` through the driver's closure, which must call
    /// `release_detached_owned_player` with the lent original handle.
    ///
    /// Only a retired, lifecycle-acknowledged record with a confirmed
    /// leave-save is released; success removes the record and frees its table
    /// entry, after which the driver removes its token (L2 ends). A refusal
    /// changes nothing.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::StaleRetention`],
    /// [`CollectorSessionError::RetentionSuperseded`],
    /// [`CollectorSessionError::RetentionNotReleasable`],
    /// [`CollectorSessionError::NativeReleaseStale`],
    /// [`CollectorSessionError::NativeReleaseRefused`] or
    /// [`CollectorSessionError::NativeReleaseFailed`].
    pub fn release_native_retention(
        &mut self,
        token: NativeRetentionToken,
        release: &mut dyn FnMut(
            &OwnedPlayerHandle,
        ) -> Result<ReleasedOwnedPlayer, PlayerOwnershipError>,
    ) -> Result<(), CollectorSessionError> {
        let slot = self
            .retention_slot(token)
            .ok_or(CollectorSessionError::StaleRetention)?;
        let original = match self.native.entry(slot).map(|entry| &entry.state) {
            Some(EntryState::Record(record)) => match record.phase {
                RecordPhase::Superseded { .. } => {
                    return Err(CollectorSessionError::RetentionSuperseded);
                }
                RecordPhase::Leaving | RecordPhase::Detached => {
                    return Err(CollectorSessionError::RetentionNotReleasable("not retired"));
                }
                RecordPhase::Retired { acked: false } => {
                    return Err(CollectorSessionError::RetentionNotReleasable(
                        "lifecycle not acknowledged",
                    ));
                }
                RecordPhase::Retired { acked: true } if !record.retention.confirmed() => {
                    return Err(CollectorSessionError::RetentionNotReleasable(
                        "leave-save not confirmed",
                    ));
                }
                RecordPhase::Retired { acked: true } => record.original.clone(),
            },
            Some(EntryState::Session(_)) => {
                return Err(CollectorSessionError::RetentionNotReleasable("not retired"));
            }
            _ => return Err(CollectorSessionError::StaleRetention),
        };
        match release(&original) {
            Ok(receipt) if receipt.player() == token.player => {
                self.native.free(slot);
                Ok(())
            }
            Err(PlayerOwnershipError::StaleHandle) => {
                Err(CollectorSessionError::NativeReleaseStale)
            }
            Err(PlayerOwnershipError::DetachedRelease(refusal)) => {
                Err(CollectorSessionError::NativeReleaseRefused(refusal))
            }
            // A receipt naming another player is as much a failure as any
            // other refusal; the record stays.
            Ok(_) | Err(_) => Err(CollectorSessionError::NativeReleaseFailed),
        }
    }

    /// .2c amendment G3: adopts a genuine admission whose connection is gone or
    /// whose `admit_native_join` was refused, consuming the reservation once.
    ///
    /// No allocation: the reservation's table entry (allocated at step 6 and
    /// counted in the session limit) becomes an unbound `Leaving` retention
    /// record holding `handle`; the lineage comes from the router's checked
    /// counter. A genuine rejoin supersedes the player's retired record.
    ///
    /// # Errors
    ///
    /// Only fatal, typed invariant breaks ([`AdoptionInvariant`]): a foreign or
    /// stale reservation, different players, a view of another handle,
    /// generation or owner lineage, an admission bound elsewhere, or an
    /// exhausted lineage. Except for a foreign or stale reservation, the
    /// reserved entry is marked `Poisoned`; the handle is neither kept nor
    /// silently dropped (the simulation player stays admitted and blocks a
    /// quiescent stop).
    pub fn adopt_unbound_native_admission(
        &mut self,
        reservation: NativeJoinReservation,
        handle: &OwnedPlayerHandle,
        view: &ConnectedOwnedPlayerView<'_>,
    ) -> Result<NativeRetentionToken, AdoptionInvariant> {
        // Consumed exactly once (§8), whatever the outcome.
        let NativeJoinReservation { router, key } = reservation;
        let player = key.player;
        let refuse = |kind| AdoptionInvariant { player, kind };
        if !router.ptr_eq(&Arc::downgrade(&self.identity)) {
            return Err(refuse(AdoptionInvariantKind::ForeignReservation));
        }
        let slot = key.slot;
        let binding = match self.native.entry(slot).map(|entry| &entry.state) {
            Some(EntryState::Reserved(record)) if record.key == key => record.binding,
            _ => return Err(refuse(AdoptionInvariantKind::StaleReservation)),
        };
        let broken = if handle.player() != player {
            Some(AdoptionInvariantKind::PlayerMismatch)
        } else if view.player() != player
            || !view.matches_owned_player(handle)
            || view.generation() != binding.generation
        {
            Some(AdoptionInvariantKind::ViewMismatch)
        } else if self.admission_bound(handle) {
            Some(AdoptionInvariantKind::AdmissionAlreadyBound)
        } else {
            None
        };
        let lineage = match broken {
            Some(kind) => Err(kind),
            None => self
                .native
                .take_lineage()
                .map_err(|_| AdoptionInvariantKind::LineageExhausted),
        };
        let lineage = match lineage {
            Ok(lineage) => lineage,
            Err(kind) => {
                if let Some(entry) = self.native.entry_mut(slot) {
                    entry.state = EntryState::Poisoned(kind);
                }
                return Err(refuse(kind));
            }
        };
        self.supersede_records(player, slot);
        if let Some(entry) = self.native.entry_mut(slot) {
            entry.state = EntryState::Record(RetainedRecord {
                key,
                original: handle.clone(),
                binding,
                bound: false,
                phase: RecordPhase::Leaving,
                retention: Retention::new(lineage),
            });
        }
        Ok(NativeRetentionToken { player, lineage })
    }

    /// G3 retirement instead of D5 for a never bound, adopted admission.
    ///
    /// Checks in the router's own state that the record was never active, that
    /// its detach fact was confirmed, and that no session or other record
    /// refers to this admission (scan bounded by the session table); then the
    /// closure's genuine view must prove the fact again. No tracking packets
    /// exist for a never bound admission. The record becomes `Retired`.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::StaleRetention`],
    /// [`CollectorSessionError::RetirementRefused`] or
    /// [`CollectorSessionError::ProbesExhausted`]; nothing changes then.
    pub fn retire_unbound_native_admission(
        &mut self,
        token: NativeRetentionToken,
        lifecycle: &mut LifecycleClosure<'_>,
    ) -> Result<(), CollectorSessionError> {
        let slot = self
            .retention_slot(token)
            .ok_or(CollectorSessionError::StaleRetention)?;
        let (original, generation) = match self.native.entry(slot).map(|entry| &entry.state) {
            Some(EntryState::Record(record)) if !record.bound => match record.phase {
                RecordPhase::Detached => (record.original.clone(), record.binding.generation),
                RecordPhase::Leaving => {
                    return Err(CollectorSessionError::RetirementRefused("not detached"));
                }
                RecordPhase::Retired { .. } | RecordPhase::Superseded { .. } => {
                    return Err(CollectorSessionError::RetirementRefused("already retired"));
                }
            },
            _ => {
                return Err(CollectorSessionError::RetirementRefused(
                    "not an adopted admission",
                ));
            }
        };
        let referenced = self.native.entries().any(|(other, entry)| {
            other != slot
                && match &entry.state {
                    EntryState::Session(session) => session.handle == original,
                    EntryState::Record(record) => record.original == original,
                    _ => false,
                }
        });
        if referenced {
            return Err(CollectorSessionError::RetirementRefused(
                "admission referenced",
            ));
        }
        let seq = self.next_probe()?;
        let verdict = lifecycle(
            &original,
            NativeLifecycleProbe {
                expected: &original,
                generation,
                seq,
                ack: false,
            },
        );
        if !(verdict.seq == seq && verdict.facts.is_some_and(|facts| facts.detached)) {
            return Err(CollectorSessionError::RetirementRefused(
                "no matching detach fact",
            ));
        }
        if let Some(EntryState::Record(record)) =
            self.native.entry_mut(slot).map(|entry| &mut entry.state)
        {
            record.phase = RecordPhase::Retired { acked: false };
        }
        Ok(())
    }

    /// Moves a detached, leaving session into a retired record in place and
    /// removes its subject baselines from every viewer (`EntityRemove` before
    /// `TabRemove`, both mandatory).
    fn retire_session(&mut self, slot: usize) {
        let Some(entry) = self.native.entry_mut(slot) else {
            return;
        };
        if !matches!(&entry.state, EntryState::Session(s) if s.retention.is_some()) {
            return;
        }
        // Temporary placeholder, overwritten below in the same step.
        let EntryState::Session(session) = std::mem::replace(
            &mut entry.state,
            EntryState::Poisoned(AdoptionInvariantKind::StaleReservation),
        ) else {
            return;
        };
        let incarnation = session.key.incarnation;
        let player = entry.player;
        let dropped = session.viewed.len();
        let Some(retention) = session.retention else {
            return;
        };
        entry.state = EntryState::Record(RetainedRecord {
            key: session.key,
            original: session.handle,
            binding: session.binding,
            bound: true,
            phase: RecordPhase::Retired { acked: false },
            retention,
        });
        self.native.baselines -= dropped;
        for viewer in 0..self.native.slots.len() {
            let Some(session) = self.native.session_mut(viewer) else {
                continue;
            };
            let Some(baseline) = session.viewed.remove(&incarnation) else {
                continue;
            };
            self.native.baselines -= 1;
            let Some(outbound) = self
                .native
                .session(viewer)
                .and_then(|s| s.outbound.as_ref())
            else {
                continue;
            };
            let mut delivered = true;
            if baseline.spawned.is_some() {
                delivered &= outbound
                    .try_send(OutboundMessage::mandatory(remove_entities_shell(&[
                        baseline.wire,
                    ])))
                    .is_ok();
            }
            delivered &= outbound
                .try_send(OutboundMessage::mandatory(player_info_remove(player)))
                .is_ok();
            if !delivered {
                self.begin_native_leave(viewer);
            }
        }
    }

    /// Marks every retired record of `player` (other than `except`) as
    /// superseded by a genuine rejoin; an already acknowledged one is removed.
    pub(super) fn supersede_records(&mut self, player: PlayerId, except: usize) {
        for slot in 0..self.native.slots.len() {
            if slot == except {
                continue;
            }
            let remove = match self.native.entry_mut(slot) {
                Some(entry) if entry.player == player => match &mut entry.state {
                    EntryState::Record(record) => match record.phase {
                        RecordPhase::Retired { acked } => {
                            record.phase = RecordPhase::Superseded { acked };
                            acked
                        }
                        _ => false,
                    },
                    _ => false,
                },
                _ => false,
            };
            if remove {
                self.native.free(slot);
            }
        }
    }

    fn retention_slot(&self, token: NativeRetentionToken) -> Option<usize> {
        self.native.entries().find_map(|(slot, entry)| {
            let lineage = match &entry.state {
                EntryState::Session(session) => session.retention.as_ref()?.lineage,
                EntryState::Record(record) => record.retention.lineage,
                _ => return None,
            };
            (entry.player == token.player && lineage == token.lineage).then_some(slot)
        })
    }

    fn retention_mut(&mut self, slot: usize) -> Option<&mut Retention> {
        match &mut self.native.entry_mut(slot)?.state {
            EntryState::Session(session) => session.retention.as_mut(),
            EntryState::Record(record) => Some(&mut record.retention),
            _ => None,
        }
    }

    fn record(&self, slot: usize) -> Option<&RetainedRecord> {
        match &self.native.entry(slot)?.state {
            EntryState::Record(record) => Some(record),
            _ => None,
        }
    }

    /// Checks before any probe that `count` more sequence values exist.
    fn reserve_probes(&self, count: usize) -> Result<(), CollectorSessionError> {
        let needed = u64::try_from(count).map_err(|_| CollectorSessionError::ProbesExhausted)?;
        self.probe_seq
            .checked_add(needed)
            .map(|_| ())
            .ok_or(CollectorSessionError::ProbesExhausted)
    }

    fn next_probe(&mut self) -> Result<u64, CollectorSessionError> {
        let seq = self.probe_seq;
        self.probe_seq = seq
            .checked_add(1)
            .ok_or(CollectorSessionError::ProbesExhausted)?;
        Ok(seq)
    }
}
