//! Native collector-session operations of [`SessionRouter`] (NET-04.2e1).
//!
//! Scope of this ticket (Root decision F3): sealed keys, the checked shared
//! wire allocator, limits and the validated registry layout, the native
//! endpoint kind with exclusivity, registration/replacement/unregistration,
//! join reservation/admission/cancel, a minimal `disconnect_native`, endpoint
//! suspension with closure revalidation, the bounded lane with movement
//! coalescing and the region pre-check. Retention records, tokens, lineage,
//! leave-save, adoption, retirement, audience/reconcile and collector
//! baselines are NET-04.2e2; native sessions therefore do not see each other
//! yet, and a leaving native session stays in the table for e2 to take over.

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
    lane_channel, region_contains, CollectorSessionError, CollectorSessionLimits, LaneItem,
    LivePose, MergeOutcome, MovementCell, NativeDisconnect, NativeEndpointState, NativeInputClass,
    NativeJoinReservation, NativeLaneIdentity, NativeRegistryLayout, NativeRevalidation,
    NativeRevalidationProbe, NativeRevalidationVerdict, NativeRouted, NativeSessionPhase,
    NativeSessionStatus, NativeShardInput, NativeShardReceiver, NativeShardRegistration,
    NativeSpawnProjection, NativeUnregisterError, PlayerSessionKey, VerdictOutcome,
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
    /// Revalidation refused the view; never rebound.
    Refused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisconnectState {
    NotRequested,
    Retained,
    Sent,
}

/// One native session. Never removed in NET-04.2e1: e2 converts a leaving
/// entry into its retention record in place.
#[derive(Debug)]
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
    #[allow(
        dead_code,
        reason = "NET-04.2e2 tab-list/audience consumes the admitted name"
    )]
    name: String,
    #[allow(
        dead_code,
        reason = "NET-04.2e2 audience consumes the admitted equipment body"
    )]
    equipment: Vec<u8>,
    disconnect: DisconnectState,
    movement: Option<Arc<MovementCell>>,
    resync_pending: bool,
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

    /// Native sessions (active or leaving) installed in this router.
    pub fn native_session_count(&self) -> usize {
        self.native_sessions.len()
    }

    /// Provisional native join reservations.
    pub fn native_reservation_count(&self) -> usize {
        self.native_reservations.len()
    }

    /// Read-only diagnostics of the session named by `key`, or `None` for a
    /// foreign, stale or unknown key.
    pub fn native_session_status(&self, key: &PlayerSessionKey) -> Option<NativeSessionStatus> {
        self.native_session(key).map(|session| NativeSessionStatus {
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
        let identity = Arc::new(NativeLaneIdentity { coverage });
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
        let identity = Arc::new(NativeLaneIdentity { coverage });
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
        for session in self.native_sessions.values_mut() {
            if session
                .binding
                .same_registration(coverage, lease.registration_id())
            {
                if session.suspension == Suspension::Bound {
                    session.suspension = Suspension::Suspended;
                }
                session.movement = None;
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
    /// [`CollectorSessionError::NativeEndpointInUse`] while any native session
    /// or reservation references the registration, or with a lease/receiver
    /// refusal. The registration is unchanged on refusal.
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
        let sessions = self
            .native_sessions
            .values()
            .filter(|s| s.binding.same_registration(coverage, id))
            .count();
        let reservations = self
            .native_reservations
            .values()
            .filter(|r| r.binding.same_registration(coverage, id))
            .count();
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
    /// allocator and counts one session-table entry. `declared_scope` and
    /// `declared_position` come from the driver's genuine native read or
    /// bootstrap plan; the position must later equal the live view bit for bit.
    /// The reservation is no simulation admission and no generation proof.
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
    /// [`CollectorSessionError::SessionIdsExhausted`] or
    /// [`CollectorSessionError::EntityIdsExhausted`].
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
        if self.has_any_session(player) {
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
        // Burn both values now: a later refusal or cancel never reissues them.
        self.next_session = incarnation.checked_add(1);
        self.next_entity_id = wire_id.checked_add(1);
        let key = PlayerSessionKey {
            router: Arc::downgrade(&self.identity),
            player,
            incarnation,
        };
        self.native_reservations.insert(
            player,
            NativeReservationRecord {
                key: key.clone(),
                wire_id,
                binding,
                position: declared_position,
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
    /// position, and validates the typed `join_game` against the layout, the
    /// live mode and seed hash, the router view distance and the configured
    /// simulation distance. Then it queues `join_game` as the session's first
    /// mandatory State packet; only that successful enqueue installs the
    /// session `Active` (and `ever_active`). The returned handle carries the
    /// session key and the [`NativeSpawnProjection`].
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::ForeignReservation`],
    /// [`CollectorSessionError::StaleReservation`],
    /// [`CollectorSessionError::StaleReceiver`],
    /// [`CollectorSessionError::AdmissionMismatch`],
    /// [`CollectorSessionError::InvalidSpawnProjection`],
    /// [`CollectorSessionError::JoinGameMismatch`] or
    /// [`CollectorSessionError::OutboundRefused`]; the reservation stays
    /// pending for the driver's cancel or adoption path.
    #[allow(
        clippy::too_many_arguments,
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
        let record = self.pending_reservation(reservation)?;
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
        if self.native_sessions.values().any(|s| s.handle == *handle) {
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
        self.native_reservations.remove(&player);
        self.native_sessions.insert(
            player,
            NativeSession {
                key: key.clone(),
                handle: handle.clone(),
                binding,
                lane: current_lane,
                phase: NativePhase::Active,
                suspension: Suspension::Bound,
                ever_active: true,
                outbound: Some(outbound),
                remote_wire_id: wire_id,
                pose: LivePose {
                    position,
                    look,
                    game_mode,
                },
                name: name.to_owned(),
                equipment,
                disconnect: DisconnectState::NotRequested,
                movement: None,
                resync_pending: false,
            },
        );
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

    /// Cancels a pending reservation and frees its counted table entry.
    ///
    /// The burned incarnation and wire id are never reissued.
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::ForeignReservation`] or
    /// [`CollectorSessionError::StaleReservation`] (already admitted or
    /// cancelled); nothing changes then.
    pub fn cancel_native_join(
        &mut self,
        reservation: NativeJoinReservation,
    ) -> Result<PlayerId, CollectorSessionError> {
        self.pending_reservation(&reservation)?;
        let NativeJoinReservation { key, .. } = reservation;
        self.native_reservations.remove(&key.player);
        Ok(key.player)
    }

    /// Starts the native leave of the session named by `key` (minimal e1 form).
    ///
    /// The session becomes `Leaving`, its own outbound lane is closed, its
    /// movement cell is closed, and exactly one `OwnedPlayerInput::disconnect`
    /// built from the stored handle is queued as a control item (it may use the
    /// reserved tail). A full or closed lane, or a suspended endpoint, retains
    /// the disconnect deduplicated; [`retry_native_disconnects`](Self::retry_native_disconnects)
    /// or revalidation sends it first. Repeated calls are idempotent. No
    /// retention token, lineage or leave-save exists yet (NET-04.2e2 extends
    /// this entry in place).
    ///
    /// # Errors
    ///
    /// [`CollectorSessionError::StaleSession`] for a foreign, stale or unknown
    /// key.
    pub fn disconnect_native(
        &mut self,
        key: &PlayerSessionKey,
    ) -> Result<NativeDisconnect, CollectorSessionError> {
        if self.native_session(key).is_none() {
            return Err(CollectorSessionError::StaleSession);
        }
        Ok(self.begin_native_leave(key.player))
    }

    /// Retries every retained native disconnect of a bound session and returns
    /// how many remain retained. Bounded by the session table.
    pub fn retry_native_disconnects(&mut self) -> usize {
        let players: Vec<PlayerId> = self
            .native_sessions
            .iter()
            .filter(|(_, s)| s.disconnect == DisconnectState::Retained)
            .map(|(player, _)| *player)
            .collect();
        players
            .into_iter()
            .filter(|player| self.flush_disconnect(*player) == NativeDisconnect::Retained)
            .count()
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
        let player = self.active_native(key)?;
        match packet {
            ServerboundPlayPacket::SetPlayerPosition(p) => self.route_native_movement(
                player,
                Some(Vec3::new(p.x(), p.y(), p.z())),
                [None, None],
            ),
            ServerboundPlayPacket::SetPlayerPositionAndRotation(p) => self.route_native_movement(
                player,
                Some(Vec3::new(p.x(), p.y(), p.z())),
                [Some(p.yaw()), Some(p.pitch())],
            ),
            ServerboundPlayPacket::SetPlayerRotation(p) => {
                self.route_native_movement(player, None, [Some(p.yaw()), Some(p.pitch())])
            }
            ServerboundPlayPacket::ServerboundSetHeldItem(p) => {
                let slot = u8::try_from(p.slot())
                    .ok()
                    .filter(|slot| *slot <= 8)
                    .ok_or(CollectorSessionError::InvalidHotbarSlot { slot: p.slot() })?;
                self.route_native_data(player, |handle| {
                    OwnedPlayerInput::select_hotbar(handle, slot)
                })
            }
            ServerboundPlayPacket::ConfirmTeleportation(_) => {
                if let Some(session) = self.native_sessions.get_mut(&player) {
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
        let player = self.active_native(key)?;
        self.route_native_data(player, |handle| {
            OwnedPlayerInput::set_game_mode(handle, mode)
        })
    }

    /// Decides whether a lane input the simulation refused at enqueue (and the
    /// driver holds with its key) may still be delivered (R2-A5).
    ///
    /// Data is live only for the exact current session while it is active;
    /// control is live for the exact current session unless revalidation found
    /// its player detached or refused its view.
    pub fn native_lane_input_live(&self, key: &PlayerSessionKey, class: NativeInputClass) -> bool {
        let Some(session) = self.native_session(key) else {
            return false;
        };
        let reachable = matches!(
            session.suspension,
            Suspension::Bound | Suspension::Suspended
        );
        match class {
            NativeInputClass::Data => reachable && session.phase == NativePhase::Active,
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
    /// a mandatory position sync to the genuine pose. A mismatching view (or a
    /// verdict of another probe) refuses revalidation: the session leaves and
    /// is never rebound. `None` marks the player detached; the session stays
    /// suspended without blocking progress. At most one closure call per
    /// session; the driver holds no handle.
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
            .try_reserve_exact(self.native_sessions.len())
            .map_err(|_| CollectorSessionError::AllocationFailed)?;
        worklist.extend(
            self.native_sessions
                .iter()
                .filter(|(_, s)| {
                    s.suspension == Suspension::Suspended
                        && s.binding.same_registration(coverage, registration_id)
                })
                .map(|(player, _)| *player),
        );
        let needed = u64::try_from(worklist.len()).unwrap_or(u64::MAX);
        if self.probe_seq.checked_add(needed).is_none() {
            return Err(CollectorSessionError::SessionIdsExhausted);
        }
        let mut report = NativeRevalidation::default();
        for player in worklist {
            let Some(handle) = self.native_sessions.get(&player).map(|s| s.handle.clone()) else {
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
                    self.rebind_native(player, &current.identity, pose);
                    report.revalidated += 1;
                }
                VerdictOutcome::Mismatch => {
                    if let Some(session) = self.native_sessions.get_mut(&player) {
                        session.suspension = Suspension::Refused;
                    }
                    self.begin_native_leave(player);
                    report.refused += 1;
                }
                VerdictOutcome::Detached => {
                    if let Some(session) = self.native_sessions.get_mut(&player) {
                        session.suspension = Suspension::Detached;
                    }
                    self.begin_native_leave(player);
                    report.detached += 1;
                }
            }
        }
        report.open = self
            .native_sessions
            .values()
            .filter(|s| {
                s.suspension == Suspension::Suspended
                    && s.binding.same_registration(coverage, registration_id)
            })
            .count();
        Ok(report)
    }

    fn rebind_native(&mut self, player: PlayerId, lane: &Arc<NativeLaneIdentity>, pose: LivePose) {
        let resync = {
            let Some(session) = self.native_sessions.get_mut(&player) else {
                return;
            };
            session.lane = Arc::clone(lane);
            session.suspension = Suspension::Bound;
            session.pose = pose;
            session.movement = None;
            std::mem::take(&mut session.resync_pending) && session.phase == NativePhase::Active
        };
        // A retained disconnect is the session's first item on the new lane.
        self.flush_disconnect(player);
        if resync && !self.send_correction(player, false) {
            self.begin_native_leave(player);
        }
    }

    fn route_native_movement(
        &mut self,
        player: PlayerId,
        position: Option<Vec3>,
        look: [Option<f32>; 2],
    ) -> Result<NativeRouted, CollectorSessionError> {
        let Some(session) = self.native_sessions.get_mut(&player) else {
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
            return Err(if self.send_correction(player, true) {
                error
            } else {
                self.begin_native_leave(player);
                CollectorSessionError::CorrectionUndeliverable
            });
        }
        let Some(session) = self.native_sessions.get_mut(&player) else {
            return Err(CollectorSessionError::StaleSession);
        };
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
        match self.send_lane(player, item, NativeInputClass::Data) {
            Ok(()) => {
                if let Some(session) = self.native_sessions.get_mut(&player) {
                    session.movement = Some(cell);
                }
                Ok(NativeRouted::Queued)
            }
            Err(failure) => Err(self.lane_refused(player, home, failure)),
        }
    }

    fn route_native_data(
        &mut self,
        player: PlayerId,
        build: impl FnOnce(OwnedPlayerHandle) -> OwnedPlayerInput,
    ) -> Result<NativeRouted, CollectorSessionError> {
        let Some(session) = self.native_sessions.get_mut(&player) else {
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
        match self.send_lane(player, item, NativeInputClass::Data) {
            Ok(()) => Ok(NativeRouted::Queued),
            Err(failure) => Err(self.lane_refused(player, home, failure)),
        }
    }

    /// Maps a refused data send to its disposition: a controlled session leave.
    fn lane_refused(
        &mut self,
        player: PlayerId,
        home: ferrumc_sim::ShardId,
        failure: LaneFailure,
    ) -> CollectorSessionError {
        if failure == LaneFailure::Superseded {
            if let Some(session) = self.native_sessions.get_mut(&player) {
                session.suspension = Suspension::Suspended;
            }
            return CollectorSessionError::EndpointSuspended;
        }
        self.begin_native_leave(player);
        match failure {
            LaneFailure::Full => CollectorSessionError::NativeLaneFull { home },
            LaneFailure::Closed | LaneFailure::Superseded => {
                CollectorSessionError::NativeLaneClosed { home }
            }
        }
    }

    /// Sets the session leaving (idempotent) and tries its disconnect.
    fn begin_native_leave(&mut self, player: PlayerId) -> NativeDisconnect {
        if let Some(session) = self.native_sessions.get_mut(&player) {
            session.phase = NativePhase::Leaving;
            session.outbound = None;
            session.movement = None;
            if session.disconnect == DisconnectState::NotRequested {
                session.disconnect = DisconnectState::Retained;
            }
        }
        self.flush_disconnect(player)
    }

    /// Sends a retained disconnect of a bound session as a control item.
    fn flush_disconnect(&mut self, player: PlayerId) -> NativeDisconnect {
        let Some(session) = self.native_sessions.get(&player) else {
            return NativeDisconnect::Retained;
        };
        match session.disconnect {
            DisconnectState::Sent => return NativeDisconnect::Sent,
            DisconnectState::NotRequested => return NativeDisconnect::Retained,
            DisconnectState::Retained => {}
        }
        if session.suspension != Suspension::Bound {
            return NativeDisconnect::Retained;
        }
        let item = LaneItem::Input(NativeShardInput {
            key: session.key.clone(),
            class: NativeInputClass::Control,
            input: OwnedPlayerInput::disconnect(session.handle.clone()),
        });
        match self.send_lane(player, item, NativeInputClass::Control) {
            Ok(()) => {
                if let Some(session) = self.native_sessions.get_mut(&player) {
                    session.disconnect = DisconnectState::Sent;
                }
                NativeDisconnect::Sent
            }
            Err(_) => NativeDisconnect::Retained,
        }
    }

    /// Non-blocking send on the session's current lane; data stops before the
    /// control reserve, control may use it. The router is the sole sender.
    fn send_lane(
        &self,
        player: PlayerId,
        item: LaneItem,
        class: NativeInputClass,
    ) -> Result<(), LaneFailure> {
        let session = self
            .native_sessions
            .get(&player)
            .ok_or(LaneFailure::Superseded)?;
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
        })
    }

    /// Queues a mandatory position sync to the last genuine pose (and, for a
    /// region correction, a droppable notice). Closes the movement cell.
    /// Returns `false` when the mandatory packet could not be queued.
    fn send_correction(&mut self, player: PlayerId, notice: bool) -> bool {
        let Some(session) = self.native_sessions.get_mut(&player) else {
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
    ) -> Result<&NativeReservationRecord, CollectorSessionError> {
        if !reservation.router.ptr_eq(&Arc::downgrade(&self.identity)) {
            return Err(CollectorSessionError::ForeignReservation);
        }
        self.native_reservations
            .get(&reservation.key.player)
            .filter(|record| record.key == reservation.key)
            .ok_or(CollectorSessionError::StaleReservation)
    }

    fn native_session(&self, key: &PlayerSessionKey) -> Option<&NativeSession> {
        self.native_sessions
            .get(&key.player)
            .filter(|session| session.key == *key)
    }

    fn active_native(&self, key: &PlayerSessionKey) -> Result<PlayerId, CollectorSessionError> {
        let session = self
            .native_session(key)
            .ok_or(CollectorSessionError::StaleSession)?;
        if session.phase != NativePhase::Active {
            return Err(CollectorSessionError::SessionNotActive);
        }
        Ok(key.player)
    }

    /// Whether `player` holds a legacy session, a native session or a
    /// reservation in this router.
    pub(super) fn has_any_session(&self, player: PlayerId) -> bool {
        self.players.contains_key(&player)
            || self.native_sessions.contains_key(&player)
            || self.native_reservations.contains_key(&player)
    }

    /// Installed legacy and native sessions (including pending leaves) plus
    /// provisional reservations; each map is bounded by the session limit.
    pub(super) fn installed_sessions(&self) -> usize {
        self.players
            .len()
            .saturating_add(self.native_sessions.len())
            .saturating_add(self.native_reservations.len())
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
