//! Native player tracking between native sessions, the genuine pose producer
//! and the read-only collector delivery disposition (NET-04.2e2, .2c
//! "Tatsächliches Collectortracking", contract A3).
//!
//! # Baselines
//!
//! Each native viewer keeps one entry per tab-listed subject session, keyed by
//! the subject's session incarnation (never a weak address). An entry is
//! created with a mandatory `TabAdd`; while the subject is inside the viewer's
//! view distance it is also spawned (mandatory `SpawnEntity`, the last
//! delivered position is the movement baseline). Leaving view sends a
//! mandatory `EntityRemove` and keeps the tab entry; re-entry spawns again.
//! The subject's retirement (D5) sends `EntityRemove` then `TabRemove` to every
//! viewer holding an entry. Entries count against the per-viewer and total
//! baseline limits before they are created.
//!
//! # Reconcile
//!
//! Every genuine pose update of a subject reconciles both directions:
//! subject → every observer and every subject → the moved viewer. Work
//! `P + 3R + 2P² + 8P + 8` (with `P` native sessions and `R = 2(P − 1)`
//! relations) and the plan's scratch bytes are admitted before the plan is
//! fallibly reserved; baseline and the subject's own outbound capacity are
//! checked before the first send. A refused reconcile changes no baseline and
//! marks the subject's tracking not current (`TrackingNotCurrent`) until the
//! next successful reconcile. A viewer that cannot accept a mandatory packet
//! is set leaving (slow-client policy) without recursion.

use std::marker::PhantomData;

use ferrumc_proto::generated::play::{ClientboundPlayPacket, GameEvent};
use ferrumc_sim::{CollectorLifecycleView, ConnectedOwnedPlayerView, ItemCollectorBinding};
use tokio::sync::mpsc::error::TrySendError;

use super::super::{relative_or_teleport_with_rotation, within_view};
use super::{
    same_bits, same_pose, Baseline, DisconnectState, EntryState, NativePhase, RecordPhase,
    SessionRouter, Suspension, SELF_WIRE_ID,
};
use crate::native::{
    CollectorAudience, CollectorDeliveryDisposition, CollectorRetirementProof,
    CollectorSessionError, CollectorViewerBinding, LivePose, NativePoseOutcome,
};
use crate::translate::{
    chunk_for_position, entity_spawn_shell, player_info_add, remove_entities_shell,
    set_equipment_shell, set_head_rotation_shell, update_entity_rotation_shell,
};
use crate::OutboundMessage;

/// `GameEvent` reason "change game mode".
const GAME_EVENT_CHANGE_MODE: u8 = 3;

/// One planned relation change of a reconcile.
#[derive(Debug, Clone, Copy)]
enum Delta {
    /// Spawn `subject` at `viewer`; `list` also adds the tab entry.
    Enter {
        viewer: usize,
        subject: usize,
        list: bool,
    },
    /// Despawn `subject` at `viewer`, keeping the tab entry.
    Leave { viewer: usize, subject: usize },
    /// Droppable movement/rotation of the reconciled subject at `viewer`.
    Move { viewer: usize },
}

impl SessionRouter {
    /// Pose producer for `PlayerChanged` (A3): updates the native session of
    /// the view's admission from the genuine simulation view (position, look,
    /// mode) and reconciles its tracking.
    ///
    /// The view must match the stored handle (owner, player, admission) and
    /// the registration's full generation; otherwise the intent is stale and
    /// nothing changes. A changed mode is presented with a mandatory
    /// `GameEvent` (change game mode). When a mandatory correction or resync
    /// is outstanding and no newer client movement was routed since, a pose
    /// that differs from the synced one (older lane items applied after the
    /// correction) gets one mandatory follow-up sync to the genuine pose. Later
    /// region corrections always use this latest genuine pose.
    ///
    /// # Errors
    ///
    /// A refused reconcile cap ([`CollectorSessionError::WorkCapacity`],
    /// [`CollectorSessionError::ScratchCapacity`],
    /// [`CollectorSessionError::BaselineCapacity`],
    /// [`CollectorSessionError::PeerBurstCapacity`],
    /// [`CollectorSessionError::AllocationFailed`]); the pose is updated, the
    /// baselines are unchanged and the subject's tracking is not current.
    pub fn native_player_changed(
        &mut self,
        view: &ConnectedOwnedPlayerView<'_>,
    ) -> Result<NativePoseOutcome, CollectorSessionError> {
        self.native_pose(view, false)
    }

    /// Pose producer for `Resync` (A3): as
    /// [`native_player_changed`](Self::native_player_changed), and an active
    /// session additionally receives a mandatory `SynchronizePlayerPosition`
    /// to the genuine pose (for example after a refused lane input).
    ///
    /// # Errors
    ///
    /// As [`native_player_changed`](Self::native_player_changed).
    pub fn native_resync(
        &mut self,
        view: &ConnectedOwnedPlayerView<'_>,
    ) -> Result<NativePoseOutcome, CollectorSessionError> {
        self.native_pose(view, true)
    }

    fn native_pose(
        &mut self,
        view: &ConnectedOwnedPlayerView<'_>,
        resync: bool,
    ) -> Result<NativePoseOutcome, CollectorSessionError> {
        let found = self.native.sessions().find_map(|(slot, session)| {
            (session.key.player == view.player()
                && view.matches_owned_player(&session.handle)
                && view.generation() == session.binding.generation)
                .then_some(slot)
        });
        let Some(slot) = found else {
            return Ok(NativePoseOutcome::Stale);
        };
        let new = LivePose {
            position: view.position(),
            look: view.look(),
            game_mode: view.game_mode(),
        };
        let Some(session) = self.native.session_mut(slot) else {
            return Ok(NativePoseOutcome::Stale);
        };
        let old = std::mem::replace(&mut session.pose, new);
        let follow_up = session
            .sync_target
            .is_some_and(|target| !same_pose(target, new));
        let active = session.phase == NativePhase::Active;
        let mut mode_delivered = true;
        if old.game_mode != new.game_mode {
            if let Some(outbound) = &session.outbound {
                let event = ClientboundPlayPacket::GameEvent(GameEvent::new(
                    GAME_EVENT_CHANGE_MODE,
                    f32::from(new.game_mode.as_id()),
                ));
                mode_delivered = outbound.try_send(OutboundMessage::mandatory(event)).is_ok();
            }
        }
        if !mode_delivered {
            self.begin_native_leave(slot);
        }
        let mut outcome = NativePoseOutcome::Updated;
        if active && mode_delivered && (resync || follow_up) {
            if self.send_correction(slot, false) {
                outcome = NativePoseOutcome::Synced;
            } else {
                self.begin_native_leave(slot);
            }
        }
        let moved = !same_bits(old.position, new.position);
        let turned = old.look[0].to_bits() != new.look[0].to_bits()
            || old.look[1].to_bits() != new.look[1].to_bits();
        self.reconcile_subject(slot, moved, turned)?;
        Ok(outcome)
    }

    /// Reconciles the tracking of `subject` in both directions; see the module
    /// documentation for the admitted work, scratch and caps.
    pub(super) fn reconcile_subject(
        &mut self,
        subject: usize,
        moved: bool,
        turned: bool,
    ) -> Result<(), CollectorSessionError> {
        let result = self.plan_reconcile(subject, moved || turned);
        let plan = match result {
            Ok(plan) => plan,
            Err(error) => {
                if let Some(session) = self.native.session_mut(subject) {
                    session.tracking_stale = true;
                }
                return Err(error);
            }
        };
        for delta in plan {
            match delta {
                Delta::Enter {
                    viewer,
                    subject: shown,
                    list,
                } => self.apply_enter(viewer, shown, list),
                Delta::Leave {
                    viewer,
                    subject: hidden,
                } => self.apply_leave(viewer, hidden),
                Delta::Move { viewer } => self.apply_move(viewer, subject, moved),
            }
        }
        if let Some(session) = self.native.session_mut(subject) {
            session.tracking_stale = false;
        }
        Ok(())
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one admitted plan: caps, both relation directions and the burst check"
    )]
    fn plan_reconcile(
        &self,
        subject: usize,
        changed: bool,
    ) -> Result<Vec<Delta>, CollectorSessionError> {
        let limits = self.collector_config()?.limits;
        let Some(s) = self.native.session(subject) else {
            return Ok(Vec::new());
        };
        let p = self.native.sessions().count();
        let r = p
            .saturating_sub(1)
            .checked_mul(2)
            .ok_or(CollectorSessionError::WorkCapacity)?;
        let work = p
            .checked_mul(p)
            .and_then(|pp| pp.checked_mul(2))
            .and_then(|w| w.checked_add(p))
            .and_then(|w| r.checked_mul(3).and_then(|r3| w.checked_add(r3)))
            .and_then(|w| p.checked_mul(8).and_then(|p8| w.checked_add(p8)))
            .and_then(|w| w.checked_add(8))
            .ok_or(CollectorSessionError::WorkCapacity)?;
        if work > limits.work().get() {
            return Err(CollectorSessionError::WorkCapacity);
        }
        let scratch = r
            .checked_mul(std::mem::size_of::<Delta>())
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Vec<Delta>>()))
            .ok_or(CollectorSessionError::ScratchCapacity)?;
        if scratch > limits.scratch().get() {
            return Err(CollectorSessionError::ScratchCapacity);
        }
        let mut plan = Vec::new();
        plan.try_reserve_exact(r)
            .map_err(|_| CollectorSessionError::AllocationFailed)?;

        let per_viewer = limits.baselines_per_viewer().get();
        let s_inc = s.key.incarnation;
        let s_active = s.phase == NativePhase::Active && s.outbound.is_some();
        let s_chunk = chunk_for_position(s.pose.position);
        let mut new_lists = 0usize;
        let mut s_new_lists = 0usize;
        let mut s_mandatory = 0usize;
        for (v, vs) in self.native.sessions() {
            if v == subject
                || !vs
                    .binding
                    .same_registration(s.binding.coverage, s.binding.registration_id)
            {
                continue;
            }
            let v_active = vs.phase == NativePhase::Active && vs.outbound.is_some();
            let in_view = within_view(
                s_chunk,
                chunk_for_position(vs.pose.position),
                self.view_distance,
            );
            if v_active {
                match vs.viewed.get(&s_inc) {
                    None if in_view && s_active => {
                        if vs.viewed.len() >= per_viewer {
                            return Err(CollectorSessionError::BaselineCapacity);
                        }
                        new_lists += 1;
                        plan.push(Delta::Enter {
                            viewer: v,
                            subject,
                            list: true,
                        });
                    }
                    Some(b) if b.spawned.is_none() && in_view && s_active => {
                        plan.push(Delta::Enter {
                            viewer: v,
                            subject,
                            list: false,
                        });
                    }
                    Some(b) if b.spawned.is_some() && !in_view => {
                        plan.push(Delta::Leave { viewer: v, subject });
                    }
                    Some(b) if b.spawned.is_some() && changed => {
                        plan.push(Delta::Move { viewer: v });
                    }
                    _ => {}
                }
            }
            if s_active && v_active {
                match s.viewed.get(&vs.key.incarnation) {
                    None if in_view => {
                        s_new_lists += 1;
                        s_mandatory += 2;
                        plan.push(Delta::Enter {
                            viewer: subject,
                            subject: v,
                            list: true,
                        });
                    }
                    Some(b) if b.spawned.is_none() && in_view => {
                        s_mandatory += 1;
                        plan.push(Delta::Enter {
                            viewer: subject,
                            subject: v,
                            list: false,
                        });
                    }
                    Some(b) if b.spawned.is_some() && !in_view => {
                        s_mandatory += 1;
                        plan.push(Delta::Leave {
                            viewer: subject,
                            subject: v,
                        });
                    }
                    _ => {}
                }
            }
        }
        if s.viewed.len().saturating_add(s_new_lists) > per_viewer {
            return Err(CollectorSessionError::BaselineCapacity);
        }
        let total = self
            .native
            .baselines
            .checked_add(new_lists)
            .and_then(|t| t.checked_add(s_new_lists))
            .ok_or(CollectorSessionError::BaselineCapacity)?;
        if total > limits.total_baselines().get() {
            return Err(CollectorSessionError::BaselineCapacity);
        }
        // The router is the sole sender of the subject's own outbound, so its
        // free capacity bounds the mandatory burst race-free.
        if let Some(outbound) = &s.outbound {
            if s_mandatory > outbound.capacity() {
                return Err(CollectorSessionError::PeerBurstCapacity);
            }
        }
        Ok(plan)
    }

    fn apply_enter(&mut self, viewer: usize, subject: usize, list: bool) {
        let Some(s) = self.native.session(subject) else {
            return;
        };
        let (inc, wire, pose) = (s.key.incarnation, s.remote_wire_id, s.pose);
        let add = list.then(|| player_info_add(s.key.player, &s.name));
        let spawn = entity_spawn_shell(
            wire,
            s.key.player,
            pose.position,
            pose.look[0],
            pose.look[1],
        );
        let equipment =
            (!s.equipment.is_empty()).then(|| set_equipment_shell(wire, s.equipment.clone()));
        let Some(v) = self.native.session_mut(viewer) else {
            return;
        };
        let Some(outbound) = &v.outbound else {
            return;
        };
        let mut delivered = true;
        if let Some(add) = add {
            delivered = outbound.try_send(OutboundMessage::mandatory(add)).is_ok();
        }
        delivered = delivered && outbound.try_send(OutboundMessage::mandatory(spawn)).is_ok();
        if delivered {
            if let Some(equipment) = equipment {
                let _ = outbound.try_send(OutboundMessage::droppable(equipment));
            }
            let fresh = v
                .viewed
                .insert(
                    inc,
                    Baseline {
                        wire,
                        spawned: Some(pose.position),
                    },
                )
                .is_none();
            if fresh {
                self.native.baselines += 1;
            }
        } else {
            self.begin_native_leave(viewer);
        }
    }

    fn apply_leave(&mut self, viewer: usize, subject: usize) {
        let Some(inc) = self.native.session(subject).map(|s| s.key.incarnation) else {
            return;
        };
        let Some(v) = self.native.session_mut(viewer) else {
            return;
        };
        let (Some(outbound), Some(baseline)) = (&v.outbound, v.viewed.get_mut(&inc)) else {
            return;
        };
        let remove = remove_entities_shell(&[baseline.wire]);
        if outbound
            .try_send(OutboundMessage::mandatory(remove))
            .is_ok()
        {
            baseline.spawned = None;
        } else {
            self.begin_native_leave(viewer);
        }
    }

    fn apply_move(&mut self, viewer: usize, subject: usize, moved: bool) {
        let Some((inc, pose)) = self
            .native
            .session(subject)
            .map(|s| (s.key.incarnation, s.pose))
        else {
            return;
        };
        let Some(v) = self.native.session_mut(viewer) else {
            return;
        };
        let (Some(outbound), Some(baseline)) = (&v.outbound, v.viewed.get_mut(&inc)) else {
            return;
        };
        let (Some(last), wire) = (baseline.spawned, baseline.wire) else {
            return;
        };
        let [yaw, pitch] = pose.look;
        let mut closed = false;
        if moved {
            let packet = relative_or_teleport_with_rotation(wire, last, pose.position, yaw, pitch);
            match outbound.try_send(OutboundMessage::droppable(packet)) {
                // The baseline advances only on a successful enqueue.
                Ok(()) => {
                    baseline.spawned = Some(pose.position);
                    closed = matches!(
                        outbound.try_send(OutboundMessage::droppable(set_head_rotation_shell(
                            wire, yaw
                        ))),
                        Err(TrySendError::Closed(_))
                    );
                }
                Err(TrySendError::Full(_)) => {}
                Err(TrySendError::Closed(_)) => closed = true,
            }
        } else {
            for packet in [
                update_entity_rotation_shell(wire, yaw, pitch),
                set_head_rotation_shell(wire, yaw),
            ] {
                closed |= matches!(
                    outbound.try_send(OutboundMessage::droppable(packet)),
                    Err(TrySendError::Closed(_))
                );
            }
        }
        if closed {
            self.begin_native_leave(viewer);
        }
    }

    /// The read-only disposition of one confirmed take's collector (.2c).
    ///
    /// `Tracked` requires the collector's current active native session (exact
    /// owner, admission and full generation), a bound endpoint, an open
    /// outbound, no requested disconnect, current tracking and the genuine
    /// `lifecycle.live()` view of the same admission; its audience is every
    /// active native viewer with a spawned baseline of the session (with the
    /// wire id that viewer knows) plus the collector itself (wire id 1).
    /// `Retired` requires the retired or superseded record of exactly this
    /// original admission and its genuine detach fact; its take audience is
    /// provably empty. Work `C + 2P + 2`, the audience limit and the scratch
    /// charge are admitted before the audience is fallibly reserved.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::CollectorBindingUnavailable`],
    /// [`CollectorSessionError::CollectorAdmissionMismatch`],
    /// [`CollectorSessionError::CollectorNotCurrent`],
    /// [`CollectorSessionError::TrackingNotCurrent`],
    /// [`CollectorSessionError::WorkCapacity`],
    /// [`CollectorSessionError::AudienceCapacity`],
    /// [`CollectorSessionError::ScratchCapacity`] or
    /// [`CollectorSessionError::AllocationFailed`]; nothing changes.
    pub fn collector_delivery(
        &self,
        collector: &ItemCollectorBinding,
        lifecycle: &CollectorLifecycleView<'_>,
    ) -> Result<CollectorDeliveryDisposition<'_>, CollectorSessionError> {
        let limits = self.collector_config()?.limits;
        let c = self.native.occupied;
        let p = self.native.sessions().count();
        let work = p
            .checked_mul(2)
            .and_then(|w| w.checked_add(c))
            .and_then(|w| w.checked_add(2))
            .ok_or(CollectorSessionError::WorkCapacity)?;
        if work > limits.work().get() {
            return Err(CollectorSessionError::WorkCapacity);
        }
        let mut mismatch = false;
        for (slot, entry) in self.native.entries() {
            if entry.player != collector.player() {
                continue;
            }
            match &entry.state {
                EntryState::Record(record)
                    if matches!(
                        record.phase,
                        RecordPhase::Retired { .. } | RecordPhase::Superseded { .. }
                    ) =>
                {
                    if !collector.matches_owned_player(&record.original)
                        || collector.generation() != record.binding.generation
                    {
                        mismatch = true;
                        continue;
                    }
                    let proven = lifecycle.matches_owned_player(&record.original)
                        && lifecycle.detach().is_some_and(|fact| {
                            fact.matches_owned_player(&record.original)
                                && fact.generation() == record.binding.generation
                        });
                    if !proven {
                        return Err(CollectorSessionError::CollectorNotCurrent);
                    }
                    return Ok(CollectorDeliveryDisposition::Retired(
                        CollectorRetirementProof {
                            player: entry.player,
                            session: record.key.clone(),
                            router: PhantomData,
                        },
                    ));
                }
                EntryState::Record(record) => {
                    if collector.matches_owned_player(&record.original) {
                        return Err(CollectorSessionError::CollectorNotCurrent);
                    }
                    mismatch = true;
                }
                EntryState::Session(session) => {
                    if !collector.matches_owned_player(&session.handle)
                        || collector.generation() != session.binding.generation
                    {
                        mismatch = true;
                        continue;
                    }
                    let live = lifecycle.matches_owned_player(&session.handle)
                        && lifecycle.live().is_some_and(|view| {
                            view.matches_owned_player(&session.handle)
                                && view.generation() == session.binding.generation
                        });
                    if session.phase != NativePhase::Active
                        || session.outbound.is_none()
                        || session.suspension != Suspension::Bound
                        || session.disconnect != DisconnectState::NotRequested
                        || !live
                    {
                        return Err(CollectorSessionError::CollectorNotCurrent);
                    }
                    if session.tracking_stale {
                        return Err(CollectorSessionError::TrackingNotCurrent);
                    }
                    return self.tracked_audience(slot, limits);
                }
                _ => {}
            }
        }
        Err(if mismatch {
            CollectorSessionError::CollectorAdmissionMismatch
        } else {
            CollectorSessionError::CollectorBindingUnavailable
        })
    }

    fn tracked_audience(
        &self,
        collector: usize,
        limits: crate::CollectorSessionLimits,
    ) -> Result<CollectorDeliveryDisposition<'_>, CollectorSessionError> {
        let Some(session) = self.native.session(collector) else {
            return Err(CollectorSessionError::CollectorBindingUnavailable);
        };
        let inc = session.key.incarnation;
        let observes = |slot: usize, viewer: &super::NativeSession| {
            slot == collector
                || (viewer.phase == NativePhase::Active
                    && viewer.outbound.is_some()
                    && viewer.viewed.get(&inc).is_some_and(|b| b.spawned.is_some()))
        };
        let count = self
            .native
            .sessions()
            .filter(|(slot, viewer)| observes(*slot, viewer))
            .count();
        if count > limits.audience().get() {
            return Err(CollectorSessionError::AudienceCapacity);
        }
        let scratch = count
            .checked_mul(std::mem::size_of::<CollectorViewerBinding>())
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<CollectorAudience<'_>>()))
            .ok_or(CollectorSessionError::ScratchCapacity)?;
        if scratch > limits.scratch().get() {
            return Err(CollectorSessionError::ScratchCapacity);
        }
        let mut viewers = Vec::new();
        viewers
            .try_reserve_exact(count)
            .map_err(|_| CollectorSessionError::AllocationFailed)?;
        for (slot, viewer) in self.native.sessions() {
            if !observes(slot, viewer) {
                continue;
            }
            let (wire, is_self) = if slot == collector {
                (SELF_WIRE_ID, true)
            } else {
                (
                    viewer.viewed.get(&inc).map_or(SELF_WIRE_ID, |b| b.wire),
                    false,
                )
            };
            viewers.push(CollectorViewerBinding {
                session: viewer.key.clone(),
                wire,
                is_self,
            });
        }
        Ok(CollectorDeliveryDisposition::Tracked(CollectorAudience {
            viewers,
            router: PhantomData,
        }))
    }
}
