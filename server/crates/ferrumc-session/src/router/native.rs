//! Native collector-session operations of [`SessionRouter`] (NET-04.2e1/e2).
//!
//! NET-04.2e1 delivered sealed keys, the checked shared wire allocator, limits
//! and the validated registry layout, the native endpoint kind with
//! exclusivity, registration/replacement/unregistration, join
//! reservation/admission/cancel, endpoint suspension with closure
//! revalidation, the bounded lane with movement coalescing and the region
//! pre-check.
//!
//! NET-04.2e2 adds the session lifecycle on one unified session table (A1,
//! R3-3): a reservation allocates the table entry that the active session, the
//! retention record and an adopted (G3) admission later take over in place, so
//! no later transition allocates. The retention, leave-save, release and
//! retirement operations live in [`retention`], the audience, reconcile and
//! pose producers in [`audience`].

mod audience;
mod retention;

use std::collections::BTreeMap;
use std::num::NonZeroU64;
use std::sync::Arc;

use ferrumc_core::{GameMode, PlayerId};
use ferrumc_math::Vec3;
use ferrumc_net::{Criticality, OutboundPriority};
use ferrumc_proto::generated::configuration::{KnownPack, RegistryData};
use ferrumc_proto::generated::play::{
    ClientboundPlayPacket, JoinGame, ServerboundPlayPacket, SynchronizePlayerPosition,
};
use ferrumc_sim::{
    ConnectedOwnedPlayerView, GenerationBinding, OwnedPlayerHandle, OwnedPlayerInput,
    ShardPartitioner,
};

use super::{EndpointKind, NativeEndpoint, PlayerSessionHandle, SessionRouter, ShardEndpoint};
use crate::directory::{ShardCoverage, ShardRegistrationId};
use crate::native::{
    lane_channel, region_contains, AdoptionInvariantKind, CollectorSessionError,
    CollectorSessionLimits, LaneItem, LaneMark, LivePose, MergeOutcome, MovementCell,
    NativeDisconnect, NativeEndpointState, NativeInputClass, NativeJoinReservation,
    NativeLaneIdentity, NativeRegistryLayout, NativeRevalidation, NativeRevalidationProbe,
    NativeRevalidationVerdict, NativeRouted, NativeSessionPhase, NativeSessionStatus,
    NativeShardInput, NativeShardReceiver, NativeShardRegistration, NativeSpawnProjection,
    NativeUnregisterError, PlayerSessionKey, Retention, VerdictOutcome, MAX_NATIVE_EQUIPMENT_BYTES,
    MAX_NATIVE_LANE_CAPACITY, MAX_SIMULATION_DISTANCE, OVERWORLD, OVERWORLD_DIMENSION_TYPE,
};
use crate::outbound::OutboundMessage;

/// Self entity id of the current app stage; remote ids come from the allocator.
const SELF_WIRE_ID: i32 = 1;
/// Largest admitted spawn-projection axis magnitude (Vanilla world bound).
const MAX_PROJECTION_AXIS: f64 = 3.0e7;
/// `JoinGame.max_players` of the validated configuration.
const JOIN_MAX_PLAYERS: i32 = 32;
/// `SpawnInfo.previous_gamemode` meaning "none".
const NO_PREVIOUS_GAMEMODE: u8 = 255;
/// `SpawnInfo.sea_level` of the flat profile.
const SEA_LEVEL: i32 = 63;
/// Droppable notice shown with every region correction.
const REGION_NOTICE: &str = "Native Simulation: Regionsgrenze (128×128 Blöcke)";
/// Longest admitted tab-list name (the protocol's username bound, H7).
const MAX_NATIVE_NAME_CHARS: usize = 16;

/// Validated collector configuration, fixed for the router's lifetime.
#[derive(Debug, Clone, Copy)]
pub(super) struct CollectorConfig {
    limits: CollectorSessionLimits,
    layout: NativeRegistryLayout,
    simulation_distance: i32,
}

/// The registration a native session or reservation refers to.
#[derive(Debug, Clone, Copy)]
pub(super) struct NativeBinding {
    coverage: ShardCoverage,
    registration_id: ShardRegistrationId,
    home: ferrumc_sim::ShardId,
    generation: GenerationBinding,
}

impl NativeBinding {
    fn same_registration(&self, coverage: ShardCoverage, id: ShardRegistrationId) -> bool {
        self.coverage == coverage && self.registration_id == id
    }
}

/// The unified native session table (A1, R3-3).
///
/// Every reservation, session, retention record and poisoned adoption owns
/// exactly one slot. Only [`reserve_slot`](Self::reserve_slot) can grow the
/// vector (fallibly, `try_reserve`); every later transition replaces the
/// slot's state in place. Its length stays bounded by the session limit
/// because freed slots are reused first.
#[derive(Debug, Default)]
pub(super) struct NativeTable {
    slots: Vec<Option<NativeEntry>>,
    occupied: usize,
    /// The next unused retention lineage; `None` once `u64::MAX` was issued.
    next_lineage: Option<NonZeroU64>,
    /// Player baselines (tab-listed subjects) across all native viewers.
    baselines: usize,
}

impl NativeTable {
    pub(super) fn new() -> Self {
        Self {
            slots: Vec::new(),
            occupied: 0,
            next_lineage: Some(NonZeroU64::MIN),
            baselines: 0,
        }
    }

    /// Finds a free slot, growing the table fallibly by one when none is free.
    fn reserve_slot(&mut self) -> Result<usize, CollectorSessionError> {
        if let Some(free) = self.slots.iter().position(Option::is_none) {
            return Ok(free);
        }
        self.slots
            .try_reserve(1)
            .map_err(|_| CollectorSessionError::AllocationFailed)?;
        self.slots.push(None);
        Ok(self.slots.len() - 1)
    }

    fn install(&mut self, slot: usize, entry: NativeEntry) {
        if let Some(cell) = self.slots.get_mut(slot) {
            if cell.is_none() {
                self.occupied += 1;
            }
            *cell = Some(entry);
        }
    }

    fn free(&mut self, slot: usize) {
        if let Some(cell) = self.slots.get_mut(slot) {
            if cell.take().is_some() {
                self.occupied -= 1;
            }
        }
    }

    fn entry(&self, slot: usize) -> Option<&NativeEntry> {
        self.slots.get(slot).and_then(Option::as_ref)
    }

    fn entry_mut(&mut self, slot: usize) -> Option<&mut NativeEntry> {
        self.slots.get_mut(slot).and_then(Option::as_mut)
    }

    fn entries(&self) -> impl Iterator<Item = (usize, &NativeEntry)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(slot, entry)| entry.as_ref().map(|entry| (slot, entry)))
    }

    fn session(&self, slot: usize) -> Option<&NativeSession> {
        match &self.entry(slot)?.state {
            EntryState::Session(session) => Some(session),
            _ => None,
        }
    }

    fn session_mut(&mut self, slot: usize) -> Option<&mut NativeSession> {
        match &mut self.entry_mut(slot)?.state {
            EntryState::Session(session) => Some(session),
            _ => None,
        }
    }

    fn sessions(&self) -> impl Iterator<Item = (usize, &NativeSession)> {
        self.entries()
            .filter_map(|(slot, entry)| match &entry.state {
                EntryState::Session(session) => Some((slot, session)),
                _ => None,
            })
    }

    fn count(&self, kind: fn(&EntryState) -> bool) -> usize {
        self.entries()
            .filter(|(_, entry)| kind(&entry.state))
            .count()
    }

    /// Issues the next lineage (checked, never reused).
    fn take_lineage(&mut self) -> Result<NonZeroU64, CollectorSessionError> {
        let lineage = self
            .next_lineage
            .ok_or(CollectorSessionError::LineageExhausted)?;
        self.next_lineage = lineage.checked_add(1);
        Ok(lineage)
    }
}

/// One occupied slot of the native session table.
#[derive(Debug)]
pub(super) struct NativeEntry {
    player: PlayerId,
    state: EntryState,
}

/// The in-place state of one table slot.
#[derive(Debug)]
#[allow(
    clippy::large_enum_variant,
    reason = "one preallocated table slot; transitions must not allocate (R3-3)"
)]
pub(super) enum EntryState {
    /// A provisional join (§5 step 6).
    Reserved(NativeReservationRecord),
    /// An active or leaving (not yet retired) native session.
    Session(NativeSession),
    /// A retention record without a mapped session: an adopted (G3) admission
    /// or a retired session that keeps its original handle until release.
    Record(RetainedRecord),
    /// A fatal adoption invariant break (§8); never freed, reported at stop.
    Poisoned(AdoptionInvariantKind),
}

/// A provisional join: counted, with burned incarnation and wire id.
#[derive(Debug)]
pub(super) struct NativeReservationRecord {
    key: PlayerSessionKey,
    wire_id: i32,
    binding: NativeBinding,
    position: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativePhase {
    Active,
    Leaving,
}

/// Endpoint binding state of a native session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Suspension {
    /// Bound to the current receiver of its registration.
    Bound,
    /// The registration was replaced; awaiting closure revalidation.
    Suspended,
    /// Revalidation found the simulation player detached; no lane is needed.
    Detached,
    /// Revalidation refused the view; never rebound. Its disconnect from the
    /// stored handle uses the current lane (A2, Root 2(d)), also after every
    /// further replacement (F2, round 2).
    Refused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisconnectState {
    NotRequested,
    Retained,
    Sent,
    /// No lane disconnect is needed (simulation player already detached).
    NotNeeded,
}

/// One tab-listed subject in a viewer's baseline map.
#[derive(Debug, Clone, Copy)]
pub(super) struct Baseline {
    /// The subject's wire id as this viewer knows it.
    wire: i32,
    /// The last delivered position while the subject is spawned at this viewer.
    spawned: Option<Vec3>,
}

/// One native session (active, or leaving until its retirement D5).
#[derive(Debug)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "independent lifecycle flags of one session entry"
)]
pub(super) struct NativeSession {
    key: PlayerSessionKey,
    handle: OwnedPlayerHandle,
    binding: NativeBinding,
    lane: Arc<NativeLaneIdentity>,
    phase: NativePhase,
    suspension: Suspension,
    ever_active: bool,
    outbound: Option<tokio::sync::mpsc::Sender<OutboundMessage>>,
    remote_wire_id: i32,
    pose: LivePose,
    name: String,
    equipment: Vec<u8>,
    disconnect: DisconnectState,
    /// Lane position of the last sent disconnect (§7.3 amendment V1, Root).
    disconnect_mark: Option<LaneMark>,
    movement: Option<Arc<MovementCell>>,
    resync_pending: bool,
    /// The genuine detach fact of the stored admission was confirmed (D3).
    detached: bool,
    /// Created by `disconnect_native` (D2) with the session lineage.
    retention: Option<Retention>,
    /// Subjects this session sees, keyed by the subject's incarnation.
    viewed: BTreeMap<NonZeroU64, Baseline>,
    /// Pose of the last mandatory sync while no newer client movement was
    /// routed; a differing genuine pose triggers one follow-up sync (A3).
    sync_target: Option<LivePose>,
    /// The last reconcile of this subject was refused by a cap.
    tracking_stale: bool,
}

/// A retention record without a mapped session.
#[derive(Debug)]
pub(super) struct RetainedRecord {
    key: PlayerSessionKey,
    original: OwnedPlayerHandle,
    binding: NativeBinding,
    bound: bool,
    phase: RecordPhase,
    retention: Retention,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordPhase {
    /// Adopted (G3); its disconnect is not applied yet.
    Leaving,
    /// Adopted (G3); the genuine detach fact was confirmed.
    Detached,
    Retired {
        acked: bool,
    },
    Superseded {
        acked: bool,
    },
}

/// Why a lane send did not enter the bounded channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaneFailure {
    Full,
    Closed,
    /// The session's lane is no longer its registration's current lane.
    Superseded,
}

impl SessionRouter {
    /// Enables native collector sessions once, before any session exists.
    ///
    /// `dimension_registry` and `core_pack` must be the exact typed records the
    /// connection advertises (the app's `ConfigRegistries`): registry
    /// `minecraft:dimension_type` with the single entry `minecraft:overworld`
    /// (index 0, no data) and the `minecraft:core:1.21.8` pack. The validated
    /// layout is `FlatOverworld1_21_8`. `simulation_distance` must lie in
    /// `0..=32` and is compared bit-exactly with every native `JoinGame`.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::AlreadyInitialized`],
    /// [`CollectorSessionError::InvalidSimulationDistance`],
    /// [`CollectorSessionError::UnsupportedRegistryLayout`],
    /// [`CollectorSessionError::InvalidLimit`] when the router's lane capacity
    /// exceeds [`MAX_NATIVE_LANE_CAPACITY`], or
    /// [`CollectorSessionError::NotEmpty`] when a session or retained leave
    /// already exists. Nothing changes on refusal.
    pub fn initialize_collector_sessions(
        &mut self,
        limits: CollectorSessionLimits,
        dimension_registry: &RegistryData,
        core_pack: &KnownPack,
        simulation_distance: i32,
    ) -> Result<(), CollectorSessionError> {
        if self.collector.is_some() {
            return Err(CollectorSessionError::AlreadyInitialized);
        }
        if !(0..=MAX_SIMULATION_DISTANCE).contains(&simulation_distance) {
            return Err(CollectorSessionError::InvalidSimulationDistance {
                distance: simulation_distance,
            });
        }
        let layout = NativeRegistryLayout::validate(dimension_registry, core_pack)?;
        if self.shard_input_capacity > MAX_NATIVE_LANE_CAPACITY {
            return Err(CollectorSessionError::InvalidLimit("native lane capacity"));
        }
        if !self.players.is_empty() || !self.pending_disconnects.is_empty() {
            return Err(CollectorSessionError::NotEmpty);
        }
        self.collector = Some(CollectorConfig {
            limits,
            layout,
            simulation_distance,
        });
        Ok(())
    }

    /// The validated collector limits, or `None` before initialization.
    pub fn collector_session_limits(&self) -> Option<CollectorSessionLimits> {
        self.collector.map(|config| config.limits)
    }

    /// Native sessions (active or leaving, not yet retired) in this router.
    pub fn native_session_count(&self) -> usize {
        self.native
            .count(|state| matches!(state, EntryState::Session(_)))
    }

    /// Provisional native join reservations.
    pub fn native_reservation_count(&self) -> usize {
        self.native
            .count(|state| matches!(state, EntryState::Reserved(_)))
    }

    /// Retention records without a mapped session (adopted or retired).
    pub fn native_record_count(&self) -> usize {
        self.native
            .count(|state| matches!(state, EntryState::Record(_)))
    }

    /// The fatal adoption invariant of `player`'s poisoned table entry, if any.
    pub fn native_poisoned(&self, player: PlayerId) -> Option<AdoptionInvariantKind> {
        self.native
            .entries()
            .find_map(|(_, entry)| match entry.state {
                EntryState::Poisoned(kind) if entry.player == player => Some(kind),
                _ => None,
            })
    }

    /// Player baselines (tab-listed subjects) across all native viewers.
    pub fn native_baseline_count(&self) -> usize {
        self.native.baselines
    }

    /// Read-only diagnostics of the session named by `key`, or `None` for a
    /// foreign, stale, retired or unknown key.
    pub fn native_session_status(&self, key: &PlayerSessionKey) -> Option<NativeSessionStatus> {
        self.native_session(key)
            .map(|(_, session)| NativeSessionStatus {
                phase: match session.phase {
                    NativePhase::Active => NativeSessionPhase::Active,
                    NativePhase::Leaving => NativeSessionPhase::Leaving,
                },
                endpoint: match session.suspension {
                    Suspension::Bound => NativeEndpointState::Bound,
                    Suspension::Suspended => NativeEndpointState::Suspended,
                    Suspension::Detached => NativeEndpointState::Detached,
                    Suspension::Refused => NativeEndpointState::RevalidationRefused,
                },
                ever_active: session.ever_active,
                disconnect: match session.disconnect {
                    DisconnectState::NotRequested => None,
                    DisconnectState::Retained => Some(NativeDisconnect::Retained),
                    DisconnectState::Sent => Some(NativeDisconnect::Sent),
                    DisconnectState::NotNeeded => Some(NativeDisconnect::NotNeeded),
                },
                remote_wire_id: session.remote_wire_id,
                self_wire_id: SELF_WIRE_ID,
                game_mode: session.pose.game_mode,
            })
    }

    /// Registers the native endpoint covering `home`'s world/dimension.
    ///
    /// `generation` must be the validated layout's scope (world 0, dimension 0,
    /// `FerrumC` flat 1.21.8 Overworld) and name `home`'s world/dimension. The
    /// lane is a bounded channel of the router's shard-input capacity.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::NotInitialized`],
    /// [`CollectorSessionError::Unsupported`],
    /// [`CollectorSessionError::EndpointKindConflict`] when legacy coverage
    /// exists in the scope, or [`CollectorSessionError::Directory`] for an
    /// occupied coverage or exhausted generation. Nothing changes on refusal.
    pub fn register_native_world_shard(
        &mut self,
        home: ferrumc_sim::ShardId,
        generation: GenerationBinding,
    ) -> Result<NativeShardRegistration, CollectorSessionError> {
        let config = self.collector_config()?;
        if !config.layout.supports(generation) {
            return Err(CollectorSessionError::Unsupported("generation"));
        }
        if generation.key().world() != home.world()
            || generation.key().dimension() != home.dimension()
        {
            return Err(CollectorSessionError::Unsupported("home scope"));
        }
        if self.scope_has_kind(home.world(), home.dimension(), false) {
            return Err(CollectorSessionError::EndpointKindConflict {
                world: home.world(),
                dimension: home.dimension(),
            });
        }
        let coverage = ShardCoverage::world(home.world(), home.dimension());
        let identity = Arc::new(NativeLaneIdentity::new(coverage));
        let (sender, receiver) = lane_channel(self.shard_input_capacity);
        let lease = self.shards.register(
            coverage,
            ShardEndpoint {
                home,
                kind: EndpointKind::Native(NativeEndpoint {
                    sender,
                    lane: Arc::clone(&identity),
                    generation,
                }),
            },
        )?;
        Ok(NativeShardRegistration {
            home,
            lease,
            receiver: NativeShardReceiver { identity, receiver },
        })
    }

    /// Replaces the lane of a native registration (same lineage and home, new
    /// generation and lane identity).
    ///
    /// Requires the current lease **and** a borrow of the current receiver.
    /// Every native session of the registration becomes `EndpointSuspended`
    /// and its movement cell is closed; the driver drains the old receiver
    /// completely before the new one, and old items keep their own handles.
    /// A session whose revalidation was refused stays refused and is never
    /// rebound, but its disconnect from the stored handle follows the new lane
    /// (F2, round 2), so a retained one still reaches the simulation.
    /// Retention records and adopted admissions do not hang on the lane.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::Directory`] for a foreign/stale lease,
    /// [`CollectorSessionError::LegacyEndpoint`] for a legacy registration, or
    /// [`CollectorSessionError::StaleReceiver`]. The endpoint stays unchanged.
    pub fn replace_native_shard_registration(
        &mut self,
        lease: &crate::ShardLease,
        current: &NativeShardReceiver,
    ) -> Result<NativeShardRegistration, CollectorSessionError> {
        let (home, generation) = {
            let entry = self.shards.get(lease)?;
            let endpoint = entry.endpoint();
            let EndpointKind::Native(native) = &endpoint.kind else {
                return Err(CollectorSessionError::LegacyEndpoint {
                    home: endpoint.home,
                });
            };
            if !Arc::ptr_eq(&native.lane, &current.identity) {
                return Err(CollectorSessionError::StaleReceiver);
            }
            (endpoint.home, native.generation)
        };
        let coverage = lease.coverage();
        let identity = Arc::new(NativeLaneIdentity::new(coverage));
        let (sender, receiver) = lane_channel(self.shard_input_capacity);
        let next = self.shards.replace(
            lease,
            ShardEndpoint {
                home,
                kind: EndpointKind::Native(NativeEndpoint {
                    sender,
                    lane: Arc::clone(&identity),
                    generation,
                }),
            },
        )?;
        for slot in 0..self.native.slots.len() {
            if let Some(session) = self.native.session_mut(slot) {
                if session
                    .binding
                    .same_registration(coverage, lease.registration_id())
                {
                    match session.suspension {
                        Suspension::Bound => session.suspension = Suspension::Suspended,
                        Suspension::Refused => session.lane = Arc::clone(&identity),
                        Suspension::Suspended | Suspension::Detached => {}
                    }
                    session.movement = None;
                }
            }
        }
        Ok(NativeShardRegistration {
            home,
            lease: next,
            receiver: NativeShardReceiver { identity, receiver },
        })
    }

    /// Removes a native registration and consumes its current receiver.
    ///
    /// # Errors
    ///
    /// Returns the receiver intact inside [`NativeUnregisterError`] with
    /// [`CollectorSessionError::NativeEndpointInUse`] while any native session,
    /// reservation, retention record, adopted admission or poisoned entry
    /// references the registration, or with a lease/receiver refusal. The
    /// registration is unchanged on refusal.
    pub fn unregister_native_shard_registration(
        &mut self,
        lease: &crate::ShardLease,
        receiver: NativeShardReceiver,
    ) -> Result<ferrumc_sim::ShardId, NativeUnregisterError> {
        if let Err(error) = self.check_native_unregister(lease, &receiver) {
            return Err(NativeUnregisterError { error, receiver });
        }
        match self.shards.remove(lease) {
            Ok(endpoint) => Ok(endpoint.home),
            Err(error) => Err(NativeUnregisterError {
                error: error.into(),
                receiver,
            }),
        }
    }

    fn check_native_unregister(
        &self,
        lease: &crate::ShardLease,
        receiver: &NativeShardReceiver,
    ) -> Result<(), CollectorSessionError> {
        let entry = self.shards.get(lease)?;
        let endpoint = entry.endpoint();
        let EndpointKind::Native(native) = &endpoint.kind else {
            return Err(CollectorSessionError::LegacyEndpoint {
                home: endpoint.home,
            });
        };
        if !Arc::ptr_eq(&native.lane, &receiver.identity) {
            return Err(CollectorSessionError::StaleReceiver);
        }
        let (coverage, id) = (lease.coverage(), lease.registration_id());
        let mut sessions = 0;
        let mut reservations = 0;
        for (_, entry) in self.native.entries() {
            match &entry.state {
                EntryState::Reserved(r) if r.binding.same_registration(coverage, id) => {
                    reservations += 1;
                }
                EntryState::Session(s) if s.binding.same_registration(coverage, id) => {
                    sessions += 1;
                }
                EntryState::Record(r) if r.binding.same_registration(coverage, id) => {
                    sessions += 1;
                }
                // A poisoned entry keeps its admitted simulation player.
                EntryState::Poisoned(_) => sessions += 1,
                _ => {}
            }
        }
        if sessions != 0 || reservations != 0 {
            return Err(CollectorSessionError::NativeEndpointInUse {
                registration_id: id,
                sessions,
                reservations,
            });
        }
        Ok(())
    }

    /// Reserves a provisional native join slot before any simulation effect.
    ///
    /// Burns one session incarnation and one wire id from the shared checked
    /// allocator and **allocates** one session-table entry (fallibly, before
    /// burning anything); the active session or an adopted admission later
    /// takes this entry over in place (A1, R3-3). `declared_scope` and
    /// `declared_position` come from the driver's genuine native read or
    /// bootstrap plan; the position must later equal the live view bit for bit.
    /// The reservation is no simulation admission and no generation proof.
    ///
    /// A retired (or superseded) retention record of the same player does not
    /// block the reservation: that is the rejoin branch before release (§7.4).
    ///
    /// # Errors
    ///
    /// All refusals precede every side effect:
    /// [`CollectorSessionError::NotInitialized`],
    /// [`CollectorSessionError::Unsupported`],
    /// [`CollectorSessionError::InvalidTransform`],
    /// [`CollectorSessionError::UnknownShard`],
    /// [`CollectorSessionError::LegacyEndpoint`],
    /// [`CollectorSessionError::AdmissionMismatch`] (scope differs from the
    /// registration), [`CollectorSessionError::NativeAdmissionOutsideRegion`],
    /// [`CollectorSessionError::DuplicatePlayer`],
    /// [`CollectorSessionError::SessionCapacity`],
    /// [`CollectorSessionError::SessionIdsExhausted`],
    /// [`CollectorSessionError::EntityIdsExhausted`] or
    /// [`CollectorSessionError::AllocationFailed`].
    pub fn reserve_native_join(
        &mut self,
        player: PlayerId,
        declared_scope: GenerationBinding,
        declared_position: Vec3,
    ) -> Result<NativeJoinReservation, CollectorSessionError> {
        let config = self.collector_config()?;
        if !config.layout.supports(declared_scope) {
            return Err(CollectorSessionError::Unsupported("declared scope"));
        }
        let chunk = ferrumc_sim::EntityTransform::try_new(declared_position, 0.0, 0.0)
            .map_err(|_| CollectorSessionError::InvalidTransform)?
            .chunk();
        let key = declared_scope.key();
        let target = ShardPartitioner::for_chunk(key.world(), key.dimension(), chunk);
        let binding = {
            let resolved = self
                .shards
                .resolve(target)
                .ok_or(CollectorSessionError::UnknownShard { target })?;
            let endpoint = resolved.endpoint();
            let EndpointKind::Native(native) = &endpoint.kind else {
                return Err(CollectorSessionError::LegacyEndpoint {
                    home: endpoint.home,
                });
            };
            if native.generation != declared_scope {
                return Err(CollectorSessionError::AdmissionMismatch("declared scope"));
            }
            NativeBinding {
                coverage: resolved.coverage(),
                registration_id: resolved.registration_id(),
                home: endpoint.home,
                generation: native.generation,
            }
        };
        if !region_contains(binding.home, declared_position)? {
            return Err(CollectorSessionError::NativeAdmissionOutsideRegion);
        }
        if self.players.contains_key(&player) || self.native_blocks_reservation(player) {
            return Err(CollectorSessionError::DuplicatePlayer { player });
        }
        let limit = config.limits.sessions().get();
        if self.installed_sessions() >= limit {
            return Err(CollectorSessionError::SessionCapacity { limit });
        }
        let incarnation = self
            .next_session
            .ok_or(CollectorSessionError::SessionIdsExhausted)?;
        let wire_id = self
            .next_entity_id
            .ok_or(CollectorSessionError::EntityIdsExhausted)?;
        let slot = self.native.reserve_slot()?;
        // Burn both values now: a later refusal or cancel never reissues them.
        self.next_session = incarnation.checked_add(1);
        self.next_entity_id = wire_id.checked_add(1);
        let key = PlayerSessionKey {
            router: Arc::downgrade(&self.identity),
            player,
            incarnation,
            slot,
        };
        self.native.install(
            slot,
            NativeEntry {
                player,
                state: EntryState::Reserved(NativeReservationRecord {
                    key: key.clone(),
                    wire_id,
                    binding,
                    position: declared_position,
                }),
            },
        );
        Ok(NativeJoinReservation {
            router: Arc::downgrade(&self.identity),
            key,
        })
    }

    /// Completes a reserved native join exactly once (.2c amendment R3).
    ///
    /// Checks, before any side effect, that the reservation is this router's
    /// and still pending, that `lane` is the current receiver of the reserved
    /// registration, that `handle` and `view` describe one genuine admission
    /// (player, exact owner/admission, full generation) at exactly the declared
    /// position, that the admission is bound nowhere else, that `name` (1 to
    /// 16 characters) and `equipment` (at most [`MAX_NATIVE_EQUIPMENT_BYTES`])
    /// are bounded, and validates the typed `join_game` against the layout, the
    /// live mode and seed hash, the router view distance and the configured
    /// simulation distance. Then it queues `join_game` as the session's first
    /// mandatory State packet and takes over the reservation's table entry in
    /// place as an `Active` session (A2, `ever_active`). The peer burst follows
    /// as one checked reconcile (tab-list add and spawn both ways with every
    /// active native session in view); its work, scratch, baseline and joiner
    /// outbound caps are checked before any send, and a refusal restores the
    /// pending reservation. A genuine rejoin supersedes the player's retired
    /// retention record in the same step (§7.4). The returned handle carries
    /// the session key and the [`NativeSpawnProjection`].
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::ForeignReservation`],
    /// [`CollectorSessionError::StaleReservation`],
    /// [`CollectorSessionError::StaleReceiver`],
    /// [`CollectorSessionError::AdmissionMismatch`],
    /// [`CollectorSessionError::InvalidJoinMetadata`],
    /// [`CollectorSessionError::InvalidSpawnProjection`],
    /// [`CollectorSessionError::JoinGameMismatch`],
    /// [`CollectorSessionError::OutboundRefused`] or a peer-burst cap
    /// ([`CollectorSessionError::PeerBurstCapacity`],
    /// [`CollectorSessionError::BaselineCapacity`],
    /// [`CollectorSessionError::WorkCapacity`],
    /// [`CollectorSessionError::ScratchCapacity`],
    /// [`CollectorSessionError::AllocationFailed`]); the reservation stays
    /// pending for the driver's cancel or adoption path.
    #[allow(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        reason = "the .2c contract fixes reservation, receiver, handle, view and the join payload"
    )]
    pub fn admit_native_join(
        &mut self,
        reservation: &NativeJoinReservation,
        lane: &NativeShardReceiver,
        handle: &OwnedPlayerHandle,
        view: &ConnectedOwnedPlayerView<'_>,
        join_game: JoinGame,
        name: &str,
        equipment: Vec<u8>,
    ) -> Result<PlayerSessionHandle, CollectorSessionError> {
        let config = self.collector_config()?;
        let (slot, record) = self.pending_reservation(reservation)?;
        let (binding, wire_id, declared) = (record.binding, record.wire_id, record.position);
        let key = record.key.clone();
        let player = key.player;
        let current_lane = {
            let entry = self
                .shards
                .current(binding.coverage)
                .ok_or(CollectorSessionError::StaleReceiver)?;
            let EndpointKind::Native(native) = &entry.endpoint().kind else {
                return Err(CollectorSessionError::StaleReceiver);
            };
            if entry.registration_id() != binding.registration_id
                || !Arc::ptr_eq(&native.lane, &lane.identity)
            {
                return Err(CollectorSessionError::StaleReceiver);
            }
            Arc::clone(&native.lane)
        };
        if handle.player() != player || view.player() != player {
            return Err(CollectorSessionError::AdmissionMismatch("player"));
        }
        if !view.matches_owned_player(handle) {
            return Err(CollectorSessionError::AdmissionMismatch("handle"));
        }
        if view.generation() != binding.generation {
            return Err(CollectorSessionError::AdmissionMismatch("generation"));
        }
        if self.admission_bound(handle) {
            return Err(CollectorSessionError::AdmissionMismatch(
                "admission already bound",
            ));
        }
        let position = view.position();
        if !same_bits(position, declared) {
            return Err(CollectorSessionError::AdmissionMismatch(
                "declared position",
            ));
        }
        let name_chars = name.chars().count();
        if name_chars == 0 || name_chars > MAX_NATIVE_NAME_CHARS {
            return Err(CollectorSessionError::InvalidJoinMetadata("name"));
        }
        if equipment.len() > MAX_NATIVE_EQUIPMENT_BYTES {
            return Err(CollectorSessionError::InvalidJoinMetadata("equipment"));
        }
        let look = view.look();
        validate_projection(position, look)?;
        self.validate_join_game(&join_game, view, config)?;

        let (outbound, receiver) = tokio::sync::mpsc::channel(self.outbound_capacity);
        outbound
            .try_send(OutboundMessage::new(
                ClientboundPlayPacket::JoinGame(join_game),
                Criticality::Mandatory,
                OutboundPriority::State,
            ))
            .map_err(|_| CollectorSessionError::OutboundRefused)?;
        let game_mode = view.game_mode();
        let pose = LivePose {
            position,
            look,
            game_mode,
        };
        let session = NativeSession {
            key: key.clone(),
            handle: handle.clone(),
            binding,
            lane: current_lane,
            phase: NativePhase::Active,
            suspension: Suspension::Bound,
            ever_active: true,
            outbound: Some(outbound),
            remote_wire_id: wire_id,
            pose,
            name: name.to_owned(),
            equipment,
            disconnect: DisconnectState::NotRequested,
            disconnect_mark: None,
            movement: None,
            resync_pending: false,
            detached: false,
            retention: None,
            viewed: BTreeMap::new(),
            sync_target: None,
            tracking_stale: false,
        };
        // Take the reservation's entry over in place; restore it on refusal.
        let previous = self
            .native
            .entry_mut(slot)
            .map(|entry| std::mem::replace(&mut entry.state, EntryState::Session(session)));
        if let Err(error) = self.reconcile_subject(slot, true, true) {
            if let (Some(entry), Some(previous)) = (self.native.entry_mut(slot), previous) {
                entry.state = previous;
            }
            return Err(error);
        }
        self.supersede_records(player, slot);
        Ok(PlayerSessionHandle {
            player,
            shard: binding.home,
            outbound: receiver,
            key: Some(key),
            projection: Some(NativeSpawnProjection {
                position,
                look,
                game_mode,
            }),
        })
    }

    /// Cancels a pending reservation and frees its allocated table entry.
    ///
    /// The burned incarnation and wire id are never reissued.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::ForeignReservation`] or
    /// [`CollectorSessionError::StaleReservation`] (already admitted, adopted
    /// or cancelled); nothing changes then.
    pub fn cancel_native_join(
        &mut self,
        reservation: NativeJoinReservation,
    ) -> Result<PlayerId, CollectorSessionError> {
        let (slot, _) = self.pending_reservation(&reservation)?;
        self.native.free(slot);
        let NativeJoinReservation { key, .. } = reservation;
        Ok(key.player)
    }

    /// Retries every retained native disconnect of a bound or refused session
    /// and returns how many remain retained. Detached sessions need no lane
    /// disconnect and never count (A2). Bounded by the session table; no
    /// allocation.
    pub fn retry_native_disconnects(&mut self) -> usize {
        let mut retained = 0;
        for slot in 0..self.native.slots.len() {
            let is_retained = self
                .native
                .session(slot)
                .is_some_and(|s| s.disconnect == DisconnectState::Retained);
            if is_retained && self.flush_disconnect(slot) == NativeDisconnect::Retained {
                retained += 1;
            }
        }
        retained
    }

    /// Routes one decoded client packet of the native session named by `key`.
    ///
    /// Movement is pre-checked with the simulation's own region derivation and
    /// coalesced per session (see the lane module); hotbar selection becomes an
    /// authoritative data item; a teleport confirmation closes the movement
    /// cell. Block, command, container and item actions are refused typed.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::StaleSession`],
    /// [`CollectorSessionError::SessionNotActive`],
    /// [`CollectorSessionError::EndpointSuspended`] (movement discarded,
    /// authoritative input answered by a resync after revalidation),
    /// [`CollectorSessionError::NativeRegionBoundary`] /
    /// [`CollectorSessionError::InvalidTransform`] (mandatory correction to the
    /// last genuine pose plus a droppable notice queued, no lane item),
    /// [`CollectorSessionError::InvalidHotbarSlot`],
    /// [`CollectorSessionError::UnsupportedNativeAction`],
    /// [`CollectorSessionError::NativeLaneFull`] /
    /// [`CollectorSessionError::NativeLaneClosed`] /
    /// [`CollectorSessionError::CorrectionUndeliverable`] (each after a
    /// controlled session leave).
    pub fn route_native_play(
        &mut self,
        key: &PlayerSessionKey,
        packet: &ServerboundPlayPacket,
    ) -> Result<NativeRouted, CollectorSessionError> {
        let slot = self.active_native(key)?;
        match packet {
            ServerboundPlayPacket::SetPlayerPosition(p) => {
                self.route_native_movement(slot, Some(Vec3::new(p.x(), p.y(), p.z())), [None, None])
            }
            ServerboundPlayPacket::SetPlayerPositionAndRotation(p) => self.route_native_movement(
                slot,
                Some(Vec3::new(p.x(), p.y(), p.z())),
                [Some(p.yaw()), Some(p.pitch())],
            ),
            ServerboundPlayPacket::SetPlayerRotation(p) => {
                self.route_native_movement(slot, None, [Some(p.yaw()), Some(p.pitch())])
            }
            ServerboundPlayPacket::ServerboundSetHeldItem(p) => {
                let selected = u8::try_from(p.slot())
                    .ok()
                    .filter(|selected| *selected <= 8)
                    .ok_or(CollectorSessionError::InvalidHotbarSlot { slot: p.slot() })?;
                self.route_native_data(slot, |handle| {
                    OwnedPlayerInput::select_hotbar(handle, selected)
                })
            }
            ServerboundPlayPacket::ConfirmTeleportation(_) => {
                if let Some(session) = self.native.session_mut(slot) {
                    session.movement = None;
                }
                Ok(NativeRouted::Barrier)
            }
            ServerboundPlayPacket::PlayerAction(_) | ServerboundPlayPacket::UseItemOn(_) => Err(
                CollectorSessionError::UnsupportedNativeAction("block action"),
            ),
            ServerboundPlayPacket::ChatCommand(_) => {
                Err(CollectorSessionError::UnsupportedNativeAction("command"))
            }
            ServerboundPlayPacket::WindowClick(_) | ServerboundPlayPacket::CloseContainer(_) => {
                Err(CollectorSessionError::UnsupportedNativeAction(
                    "container action",
                ))
            }
            ServerboundPlayPacket::SetCreativeSlot(_) | ServerboundPlayPacket::UpdateSign(_) => {
                Err(CollectorSessionError::UnsupportedNativeAction(
                    "item action",
                ))
            }
            _ => Ok(NativeRouted::Ignored),
        }
    }

    /// Routes a native game-mode change (for example from a command) as an
    /// authoritative data item built from the stored handle.
    ///
    /// # Errors
    ///
    /// As [`route_native_play`](Self::route_native_play) for authoritative data.
    pub fn route_native_game_mode(
        &mut self,
        key: &PlayerSessionKey,
        mode: GameMode,
    ) -> Result<NativeRouted, CollectorSessionError> {
        let slot = self.active_native(key)?;
        self.route_native_data(slot, |handle| OwnedPlayerInput::set_game_mode(handle, mode))
    }

    /// Decides whether a lane input the simulation refused at enqueue (and the
    /// driver holds with its key) may still be delivered (R2-A5).
    ///
    /// Data is live only for the exact current session while it is active and
    /// reachable; control is live for the exact current session while its
    /// endpoint is bound, suspended or refused (A2) and the genuine detach fact
    /// of its admission is not yet confirmed (A4). Retired sessions have no key
    /// mapping and are never live.
    pub fn native_lane_input_live(&self, key: &PlayerSessionKey, class: NativeInputClass) -> bool {
        let Some((_, session)) = self.native_session(key) else {
            return false;
        };
        let reachable = matches!(
            session.suspension,
            Suspension::Bound | Suspension::Suspended | Suspension::Refused
        ) && !session.detached;
        match class {
            NativeInputClass::Data => {
                reachable
                    && session.suspension != Suspension::Refused
                    && session.phase == NativePhase::Active
            }
            NativeInputClass::Control => reachable,
        }
    }

    /// Revalidates suspended native sessions of `current`'s registration
    /// (Root R3-A1, .2c amendment R4).
    ///
    /// For each suspended session (active or leaving) the closure receives the
    /// **stored** handle and a single-use probe; it derives the live view from
    /// its shard borrow and returns `probe.submit(view)`. A matching view
    /// rebinds the session to `current`, sends a retained disconnect as its
    /// first lane item and, if authoritative input was refused while suspended,
    /// a mandatory position sync to the genuine pose (a later differing pose
    /// from older lane items triggers one follow-up sync, A3). A mismatching
    /// view (or a verdict of another probe) refuses revalidation: the session
    /// leaves, is never rebound to another admission, and its disconnect built
    /// from the stored handle goes to the current lane, where the simulation
    /// either applies it (normal detach path) or refuses it (the record then
    /// ends in typed quarantine, A2). `None` marks the player detached: no lane
    /// disconnect is retained. At most one closure call per session; the
    /// driver holds no handle.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::StaleReceiver`] when `current` is not the
    /// current receiver, [`CollectorSessionError::AllocationFailed`] for the
    /// bounded worklist, or [`CollectorSessionError::SessionIdsExhausted`] when
    /// the probe sequence is exhausted. No session changes on these refusals.
    pub fn revalidate_native_endpoints(
        &mut self,
        current: &NativeShardReceiver,
        probe: &mut dyn FnMut(
            &OwnedPlayerHandle,
            NativeRevalidationProbe<'_>,
        ) -> NativeRevalidationVerdict,
    ) -> Result<NativeRevalidation, CollectorSessionError> {
        let (coverage, registration_id, generation) = {
            let entry = self
                .shards
                .current(current.identity.coverage)
                .ok_or(CollectorSessionError::StaleReceiver)?;
            let EndpointKind::Native(native) = &entry.endpoint().kind else {
                return Err(CollectorSessionError::StaleReceiver);
            };
            if !Arc::ptr_eq(&native.lane, &current.identity) {
                return Err(CollectorSessionError::StaleReceiver);
            }
            (
                current.identity.coverage,
                entry.registration_id(),
                native.generation,
            )
        };
        let mut worklist = Vec::new();
        worklist
            .try_reserve_exact(self.native.slots.len())
            .map_err(|_| CollectorSessionError::AllocationFailed)?;
        worklist.extend(
            self.native
                .sessions()
                .filter(|(_, s)| {
                    s.suspension == Suspension::Suspended
                        && s.binding.same_registration(coverage, registration_id)
                })
                .map(|(slot, _)| slot),
        );
        let needed = u64::try_from(worklist.len()).unwrap_or(u64::MAX);
        if self.probe_seq.checked_add(needed).is_none() {
            return Err(CollectorSessionError::SessionIdsExhausted);
        }
        let mut report = NativeRevalidation::default();
        for slot in worklist {
            let Some(handle) = self.native.session(slot).map(|s| s.handle.clone()) else {
                continue;
            };
            let seq = self.probe_seq;
            self.probe_seq += 1;
            let verdict = probe(
                &handle,
                NativeRevalidationProbe {
                    expected: &handle,
                    generation,
                    seq,
                },
            );
            let outcome = if verdict.seq == seq {
                verdict.outcome
            } else {
                VerdictOutcome::Mismatch
            };
            match outcome {
                VerdictOutcome::Matched(pose) => {
                    self.rebind_native(slot, &current.identity, pose);
                    report.revalidated += 1;
                }
                VerdictOutcome::Mismatch => {
                    if let Some(session) = self.native.session_mut(slot) {
                        session.suspension = Suspension::Refused;
                        // Never rebound: only the stored handle's disconnect
                        // may use the current lane.
                        session.lane = Arc::clone(&current.identity);
                    }
                    self.begin_native_leave(slot);
                    report.refused += 1;
                }
                VerdictOutcome::Detached => {
                    if let Some(session) = self.native.session_mut(slot) {
                        session.suspension = Suspension::Detached;
                    }
                    self.begin_native_leave(slot);
                    report.detached += 1;
                }
            }
        }
        report.open = self
            .native
            .sessions()
            .filter(|(_, s)| {
                s.suspension == Suspension::Suspended
                    && s.binding.same_registration(coverage, registration_id)
            })
            .count();
        Ok(report)
    }

    fn rebind_native(&mut self, slot: usize, lane: &Arc<NativeLaneIdentity>, pose: LivePose) {
        let resync = {
            let Some(session) = self.native.session_mut(slot) else {
                return;
            };
            session.lane = Arc::clone(lane);
            session.suspension = Suspension::Bound;
            session.pose = pose;
            session.movement = None;
            std::mem::take(&mut session.resync_pending) && session.phase == NativePhase::Active
        };
        // A retained disconnect is the session's first item on the new lane.
        self.flush_disconnect(slot);
        if resync && !self.send_correction(slot, false) {
            self.begin_native_leave(slot);
        }
    }

    fn route_native_movement(
        &mut self,
        slot: usize,
        position: Option<Vec3>,
        look: [Option<f32>; 2],
    ) -> Result<NativeRouted, CollectorSessionError> {
        let Some(session) = self.native.session_mut(slot) else {
            return Err(CollectorSessionError::StaleSession);
        };
        if session.suspension != Suspension::Bound {
            // Coalescible movement is discarded explicitly while suspended.
            return Err(CollectorSessionError::EndpointSuspended);
        }
        let home = session.binding.home;
        let finite_look = look.iter().flatten().all(|angle| angle.is_finite());
        let verdict = match position {
            _ if !finite_look => Err(CollectorSessionError::InvalidTransform),
            Some(position) => match region_contains(home, position) {
                Ok(true) => Ok(()),
                Ok(false) => Err(CollectorSessionError::NativeRegionBoundary),
                Err(error) => Err(error),
            },
            None => Ok(()),
        };
        if let Err(error) = verdict {
            return Err(if self.send_correction(slot, true) {
                error
            } else {
                self.begin_native_leave(slot);
                CollectorSessionError::CorrectionUndeliverable
            });
        }
        let Some(session) = self.native.session_mut(slot) else {
            return Err(CollectorSessionError::StaleSession);
        };
        // Newer client movement: the last sync is no longer the target (A3).
        session.sync_target = None;
        if let Some(cell) = &session.movement {
            match cell.merge(position, look) {
                MergeOutcome::Merged => return Ok(NativeRouted::Coalesced),
                MergeOutcome::Taken | MergeOutcome::Poisoned => session.movement = None,
            }
        }
        let cell = Arc::new(MovementCell::new(session.handle.clone(), position, look));
        let item = LaneItem::Movement {
            key: session.key.clone(),
            cell: Arc::clone(&cell),
        };
        match self.send_lane(slot, item, NativeInputClass::Data) {
            Ok(_) => {
                if let Some(session) = self.native.session_mut(slot) {
                    session.movement = Some(cell);
                }
                Ok(NativeRouted::Queued)
            }
            Err(failure) => Err(self.lane_refused(slot, home, failure)),
        }
    }

    fn route_native_data(
        &mut self,
        slot: usize,
        build: impl FnOnce(OwnedPlayerHandle) -> OwnedPlayerInput,
    ) -> Result<NativeRouted, CollectorSessionError> {
        let Some(session) = self.native.session_mut(slot) else {
            return Err(CollectorSessionError::StaleSession);
        };
        // Every non-movement item is a barrier: later movement queues behind it.
        session.movement = None;
        if session.suspension != Suspension::Bound {
            session.resync_pending = true;
            return Err(CollectorSessionError::EndpointSuspended);
        }
        let home = session.binding.home;
        let item = LaneItem::Input(NativeShardInput {
            key: session.key.clone(),
            class: NativeInputClass::Data,
            input: build(session.handle.clone()),
        });
        match self.send_lane(slot, item, NativeInputClass::Data) {
            Ok(_) => Ok(NativeRouted::Queued),
            Err(failure) => Err(self.lane_refused(slot, home, failure)),
        }
    }

    /// Maps a refused data send to its disposition: a controlled session leave.
    fn lane_refused(
        &mut self,
        slot: usize,
        home: ferrumc_sim::ShardId,
        failure: LaneFailure,
    ) -> CollectorSessionError {
        if failure == LaneFailure::Superseded {
            if let Some(session) = self.native.session_mut(slot) {
                session.suspension = Suspension::Suspended;
            }
            return CollectorSessionError::EndpointSuspended;
        }
        self.begin_native_leave(slot);
        match failure {
            LaneFailure::Full => CollectorSessionError::NativeLaneFull { home },
            LaneFailure::Closed | LaneFailure::Superseded => {
                CollectorSessionError::NativeLaneClosed { home }
            }
        }
    }

    /// Sets the session leaving (idempotent): closes its own outbound and
    /// movement cell, drops its viewer baselines and tries its disconnect. A
    /// session whose simulation player is known detached needs none.
    fn begin_native_leave(&mut self, slot: usize) -> NativeDisconnect {
        let mut dropped = 0;
        if let Some(session) = self.native.session_mut(slot) {
            session.phase = NativePhase::Leaving;
            session.outbound = None;
            session.movement = None;
            dropped = session.viewed.len();
            session.viewed.clear();
            if session.disconnect == DisconnectState::NotRequested {
                session.disconnect =
                    if session.suspension == Suspension::Detached || session.detached {
                        DisconnectState::NotNeeded
                    } else {
                        DisconnectState::Retained
                    };
            }
        }
        self.native.baselines -= dropped;
        self.flush_disconnect(slot)
    }

    /// Sends a retained disconnect of a bound or refused session as a control
    /// item built from the stored handle and remembers its lane position.
    fn flush_disconnect(&mut self, slot: usize) -> NativeDisconnect {
        let Some(session) = self.native.session(slot) else {
            return NativeDisconnect::Retained;
        };
        match session.disconnect {
            DisconnectState::Sent => return NativeDisconnect::Sent,
            DisconnectState::NotNeeded => return NativeDisconnect::NotNeeded,
            DisconnectState::NotRequested => return NativeDisconnect::Retained,
            DisconnectState::Retained => {}
        }
        if session.detached {
            if let Some(session) = self.native.session_mut(slot) {
                session.disconnect = DisconnectState::NotNeeded;
            }
            return NativeDisconnect::NotNeeded;
        }
        if !matches!(session.suspension, Suspension::Bound | Suspension::Refused) {
            return NativeDisconnect::Retained;
        }
        let item = LaneItem::Input(NativeShardInput {
            key: session.key.clone(),
            class: NativeInputClass::Control,
            input: OwnedPlayerInput::disconnect(session.handle.clone()),
        });
        match self.send_lane(slot, item, NativeInputClass::Control) {
            Ok(position) => {
                if let Some(session) = self.native.session_mut(slot) {
                    session.disconnect = DisconnectState::Sent;
                    session.disconnect_mark = Some(LaneMark::new(&session.lane, position));
                }
                NativeDisconnect::Sent
            }
            Err(_) => NativeDisconnect::Retained,
        }
    }

    /// Non-blocking send on the session's current lane; data stops before the
    /// control reserve, control may use it. The router is the sole sender.
    /// Returns the item's lane position (§7.3 amendment V1).
    fn send_lane(
        &self,
        slot: usize,
        item: LaneItem,
        class: NativeInputClass,
    ) -> Result<u64, LaneFailure> {
        let session = self.native.session(slot).ok_or(LaneFailure::Superseded)?;
        let entry = self
            .shards
            .current(session.binding.coverage)
            .ok_or(LaneFailure::Superseded)?;
        let EndpointKind::Native(native) = &entry.endpoint().kind else {
            return Err(LaneFailure::Superseded);
        };
        if entry.registration_id() != session.binding.registration_id
            || !Arc::ptr_eq(&native.lane, &session.lane)
        {
            return Err(LaneFailure::Superseded);
        }
        if class == NativeInputClass::Data && native.sender.capacity() <= self.shard_control_reserve
        {
            return Err(LaneFailure::Full);
        }
        native.sender.try_send(item).map_err(|error| match error {
            tokio::sync::mpsc::error::TrySendError::Full(_) => LaneFailure::Full,
            tokio::sync::mpsc::error::TrySendError::Closed(_) => LaneFailure::Closed,
        })?;
        Ok(native.lane.record_send())
    }

    /// Queues a mandatory position sync to the last genuine pose (and, for a
    /// region correction, a droppable notice). Closes the movement cell and
    /// records the pose as the sync target for the A3 follow-up.
    ///
    /// The sync carries teleport id 0 as a placeholder: the only path out of
    /// the router is the session's outbound channel, and the connection stamps
    /// every `SynchronizePlayerPosition` from it with its own `PendingTeleport`
    /// id authority (`mirror_outbound`) and withholds client positions until
    /// the matching confirmation (Root decision 2(b), A6 allocation in the app).
    /// Returns `false` when the mandatory packet could not be queued.
    fn send_correction(&mut self, slot: usize, notice: bool) -> bool {
        let Some(session) = self.native.session_mut(slot) else {
            return false;
        };
        session.movement = None;
        let Some(outbound) = &session.outbound else {
            return false;
        };
        let pose = session.pose;
        let sync =
            ClientboundPlayPacket::SynchronizePlayerPosition(SynchronizePlayerPosition::new(
                0,
                pose.position.x,
                pose.position.y,
                pose.position.z,
                0.0,
                0.0,
                0.0,
                pose.look[0],
                pose.look[1],
                0,
            ));
        if outbound.try_send(OutboundMessage::mandatory(sync)).is_err() {
            return false;
        }
        if notice {
            let text = ferrumc_core::TextComponent::text(REGION_NOTICE);
            let _ = outbound.try_send(OutboundMessage::droppable(crate::action_bar(&text)));
        }
        session.sync_target = Some(pose);
        true
    }

    fn validate_join_game(
        &self,
        join_game: &JoinGame,
        view: &ConnectedOwnedPlayerView<'_>,
        config: CollectorConfig,
    ) -> Result<(), CollectorSessionError> {
        use CollectorSessionError::JoinGameMismatch as Mismatch;
        let world = join_game.world_state();
        let checks: [(bool, &'static str); 21] = [
            (join_game.entity_id() == SELF_WIRE_ID, "self entity id"),
            (!join_game.is_hardcore(), "hardcore"),
            (
                join_game.world_names().len() == 1
                    && join_game
                        .world_names()
                        .first()
                        .is_some_and(|name| name.as_str() == OVERWORLD),
                "world names",
            ),
            (join_game.max_players() == JOIN_MAX_PLAYERS, "max players"),
            (
                join_game.view_distance() == self.view_distance,
                "view distance",
            ),
            (
                join_game.simulation_distance() == config.simulation_distance,
                "simulation distance",
            ),
            (!join_game.reduced_debug_info(), "reduced debug info"),
            (join_game.enable_respawn_screen(), "respawn screen"),
            (!join_game.do_limited_crafting(), "limited crafting"),
            (!join_game.enforces_secure_chat(), "secure chat"),
            (
                world.dimension_type() == OVERWORLD_DIMENSION_TYPE,
                "dimension type",
            ),
            (
                world.dimension_name().as_str() == OVERWORLD,
                "dimension name",
            ),
            (world.hashed_seed() == view.hashed_seed(), "hashed seed"),
            (world.gamemode() == view.game_mode().as_id(), "game mode"),
            (
                world.previous_gamemode() == NO_PREVIOUS_GAMEMODE,
                "previous game mode",
            ),
            (!world.is_debug(), "debug world"),
            (world.is_flat(), "flat world"),
            (world.death_location().is_none(), "death location"),
            (world.portal_cooldown() == 0, "portal cooldown"),
            (world.sea_level() == SEA_LEVEL, "sea level"),
            (
                config.layout == NativeRegistryLayout::FlatOverworld1_21_8,
                "layout",
            ),
        ];
        match checks.iter().find(|(ok, _)| !ok) {
            Some((_, field)) => Err(Mismatch(field)),
            None => Ok(()),
        }
    }

    fn collector_config(&self) -> Result<CollectorConfig, CollectorSessionError> {
        self.collector.ok_or(CollectorSessionError::NotInitialized)
    }

    fn pending_reservation(
        &self,
        reservation: &NativeJoinReservation,
    ) -> Result<(usize, &NativeReservationRecord), CollectorSessionError> {
        if !reservation.router.ptr_eq(&Arc::downgrade(&self.identity)) {
            return Err(CollectorSessionError::ForeignReservation);
        }
        let slot = reservation.key.slot;
        match self.native.entry(slot).map(|entry| &entry.state) {
            Some(EntryState::Reserved(record)) if record.key == reservation.key => {
                Ok((slot, record))
            }
            _ => Err(CollectorSessionError::StaleReservation),
        }
    }

    /// The mapped session of exactly `key` (active or leaving, not retired).
    fn native_session(&self, key: &PlayerSessionKey) -> Option<(usize, &NativeSession)> {
        self.native
            .session(key.slot)
            .filter(|session| session.key == *key)
            .map(|session| (key.slot, session))
    }

    fn active_native(&self, key: &PlayerSessionKey) -> Result<usize, CollectorSessionError> {
        let (slot, session) = self
            .native_session(key)
            .ok_or(CollectorSessionError::StaleSession)?;
        if session.phase != NativePhase::Active {
            return Err(CollectorSessionError::SessionNotActive);
        }
        Ok(slot)
    }

    /// Whether a native entry of `player` blocks a new reservation: every
    /// state except a retired or superseded retention record (the rejoin
    /// branch before release, §7.4).
    fn native_blocks_reservation(&self, player: PlayerId) -> bool {
        self.native.entries().any(|(_, entry)| {
            entry.player == player
                && !matches!(
                    &entry.state,
                    EntryState::Record(RetainedRecord {
                        phase: RecordPhase::Retired { .. } | RecordPhase::Superseded { .. },
                        ..
                    })
                )
        })
    }

    /// Whether `handle`'s admission is already held by a session or record.
    fn admission_bound(&self, handle: &OwnedPlayerHandle) -> bool {
        self.native.entries().any(|(_, entry)| match &entry.state {
            EntryState::Session(session) => session.handle == *handle,
            EntryState::Record(record) => record.original == *handle,
            _ => false,
        })
    }

    /// Whether `player` holds a legacy session or any native table entry in
    /// this router.
    pub(super) fn has_any_session(&self, player: PlayerId) -> bool {
        self.players.contains_key(&player)
            || self
                .native
                .entries()
                .any(|(_, entry)| entry.player == player)
    }

    /// Installed legacy sessions (including pending leaves) plus every native
    /// table entry (reservations, sessions, retention records, poisoned).
    pub(super) fn installed_sessions(&self) -> usize {
        self.players.len().saturating_add(self.native.occupied)
    }

    /// Whether `world`/`dimension` holds coverage of the given kind.
    pub(super) fn scope_has_kind(
        &self,
        world: ferrumc_core::WorldId,
        dimension: ferrumc_core::DimensionId,
        native: bool,
    ) -> bool {
        self.shards.registrations().any(|entry| {
            let (w, d) = match entry.lease().coverage() {
                ShardCoverage::Exact(shard) => (shard.world(), shard.dimension()),
                ShardCoverage::World { world, dimension } => (world, dimension),
            };
            w == world
                && d == dimension
                && matches!(entry.endpoint().kind, EndpointKind::Native(_)) == native
        })
    }
}

fn same_bits(a: Vec3, b: Vec3) -> bool {
    a.x.to_bits() == b.x.to_bits()
        && a.y.to_bits() == b.y.to_bits()
        && a.z.to_bits() == b.z.to_bits()
}

fn same_pose(a: LivePose, b: LivePose) -> bool {
    same_bits(a.position, b.position)
        && a.look[0].to_bits() == b.look[0].to_bits()
        && a.look[1].to_bits() == b.look[1].to_bits()
}

fn validate_projection(position: Vec3, look: [f32; 2]) -> Result<(), CollectorSessionError> {
    for axis in [position.x, position.y, position.z] {
        if !axis.is_finite() || axis.abs() > MAX_PROJECTION_AXIS {
            return Err(CollectorSessionError::InvalidSpawnProjection("position"));
        }
    }
    if !look.iter().all(|angle| angle.is_finite()) {
        return Err(CollectorSessionError::InvalidSpawnProjection("look"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;
