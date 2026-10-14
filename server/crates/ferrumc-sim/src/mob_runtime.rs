//! Bounded legacy mob consumer; canonical `EntityStore` registrations alone own transforms.

use crate::mobs::{MobController, MobError};
use crate::{EntityData, EntityHandle, EntityStore, MobKind, MobRules, MobSpawnLight, ShardId};
use ferrumc_core::{EntityId, EntityUuid, PlayerId};
use ferrumc_math::Vec3;
use std::collections::{BTreeMap, VecDeque};

pub(crate) const MAX_MOB_EVENTS: usize = 256;
pub(crate) const MAX_MOB_SPAWNS: usize = 16;

/// Refusal before a complete mob gameplay operation commits.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MobRuntimeError {
    /// A live legacy consumer is required; native durability stays exclusive.
    #[error("mob runtime ownership unavailable")]
    OwnershipRequired,
    /// Mob selection may occur only once.
    #[error("mobs already initialized")]
    AlreadyInitialized,
    /// Runtime has not been selected.
    #[error("mobs not initialized")]
    NotInitialized,
    /// Explicit mob, pending spawn, event, regional or player-profile bound is full.
    #[error("mob runtime capacity: {0}")]
    Capacity(&'static str),
    /// Supplied identity conflicts with any active or reserved gameplay identity.
    #[error("invalid or duplicate gameplay entity identity")]
    InvalidIdentity,
    /// Player, position, source or scope does not belong to this runtime.
    #[error("invalid mob gameplay input")]
    InvalidInput,
    /// Canonical entity owner refused before mutation.
    #[error(transparent)]
    Entity(#[from] crate::EntityStoreError),
    /// Pinned subset terrain, light, time or health rule refused.
    #[error(transparent)]
    Rules(#[from] MobError),
    /// Native mob body is malformed or disagrees with its outer envelope.
    #[error("invalid native mob record: {0}")]
    NativeRecord(&'static str),
    /// Opaque bounded storage record construction failed.
    #[error(transparent)]
    Storage(#[from] ferrumc_storage::StorageError),
    /// Complete death loot could not be reserved; health stays unchanged.
    #[error("mob death loot reservation refused")]
    LootCapacity,
}

/// Fresh process-global wire/UUID pair supplied by the driver, never minted here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobIdentity {
    pub(crate) uuid: EntityUuid,
    pub(crate) wire: EntityId,
}
impl MobIdentity {
    /// Creates an identity proposal; actual duplicate/bounds checks occur at owner admission.
    pub const fn new(uuid: EntityUuid, wire: EntityId) -> Self {
        Self { uuid, wire }
    }
    /// Original globally allocated persistent UUID.
    pub const fn uuid(self) -> EntityUuid {
        self.uuid
    }
    /// Original globally allocated nonplayer wire registration.
    pub const fn wire_id(self) -> EntityId {
        self.wire
    }
}

/// One bounded natural spawn proposal from verified resident App lighting.
#[derive(Debug, Clone, Copy)]
pub struct MobSpawnAttempt {
    pub(crate) identity: MobIdentity,
    pub(crate) kind: MobKind,
    pub(crate) position: Vec3,
    pub(crate) light: MobSpawnLight,
    pub(crate) sky_roll: u8,
    pub(crate) dark_roll: u8,
    pub(crate) seed: u64,
    pub(crate) admitted_revision: u64,
}
impl MobSpawnAttempt {
    /// Builds a proposal; the live owner rechecks genuine player distance, loaded
    /// terrain, obstruction, border, shared-spawn exclusion and population caps.
    /// Light must come from App's current validated resident column, never a guess.
    pub const fn new(
        identity: MobIdentity,
        kind: MobKind,
        position: Vec3,
        light: MobSpawnLight,
        sky_roll: u8,
        dark_roll: u8,
        seed: u64,
    ) -> Self {
        Self {
            identity,
            kind,
            position,
            light,
            sky_roll,
            dark_roll,
            seed,
            admitted_revision: 0,
        }
    }
    /// The identity is retained on any refused admission.
    pub const fn identity(self) -> MobIdentity {
        self.identity
    }
}

/// Complete spawn proposal retained on enqueue refusal, with no consumed identity.
#[derive(Debug)]
pub struct MobSpawnRefusal {
    pub(crate) reason: MobRuntimeError,
    pub(crate) attempt: MobSpawnAttempt,
}
impl MobSpawnRefusal {
    /// Classified admission error.
    pub const fn reason(&self) -> &MobRuntimeError {
        &self.reason
    }
    /// Returns the unchanged complete proposal for retry or explicit retirement.
    pub fn into_parts(self) -> (MobRuntimeError, MobSpawnAttempt) {
        (self.reason, self.attempt)
    }
}

/// Fixed-size read-only projection of the actual canonical transform and health owner.
#[derive(Debug, Clone, Copy)]
pub struct MobSnapshot {
    pub(crate) base: EntityData,
    pub(crate) kind: MobKind,
    pub(crate) health: f32,
}
impl MobSnapshot {
    /// Immutable genuine UUID/wire/type/owner/transform registration.
    pub const fn base(self) -> EntityData {
        self.base
    }
    /// Admitted adult mob form.
    pub const fn kind(self) -> MobKind {
        self.kind
    }
    /// Current controller-owned actual health, including a final zero-health edge.
    pub const fn health(self) -> f32 {
        self.health
    }
}

/// Bounded fixed-size publication; no mutable entity or item payload is duplicated.
#[derive(Debug)]
pub enum MobEvent {
    /// Genuine committed canonical registration and initial health.
    Spawn(MobSnapshot),
    /// Genuine committed canonical position/yaw update.
    Move(EntityData),
    /// Actual positive damage changed authoritative health.
    Hurt(MobSnapshot),
    /// Canonical owner registration was removed after reserved death loot admission.
    Remove(EntityData),
    /// Natural spawn failed at its tick; original identity remains explicit.
    SpawnRefused {
        /// Unconsumed original identity.
        identity: MobIdentity,
        /// Classified complete admission refusal.
        reason: MobRuntimeError,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MobPlayerProfile {
    pub(crate) targetable: bool,
    pub(crate) wire: EntityId,
    pub(crate) armor: f32,
    pub(crate) toughness: f32,
}

#[derive(Debug)]
pub(crate) struct MobRuntime {
    pub(crate) live: bool,
    pub(crate) world: ferrumc_core::WorldId,
    pub(crate) dimension: ferrumc_core::DimensionId,
    pub(crate) now: u64,
    pub(crate) peaceful: bool,
    pub(crate) shared_spawn: Vec3,
    pub(crate) rules: MobRules,
    pub(crate) controller: Option<MobController>,
    pub(crate) stores: BTreeMap<ShardId, EntityStore>,
    pub(crate) active: BTreeMap<EntityId, EntityHandle>,
    pub(crate) pending: VecDeque<MobSpawnAttempt>,
    pub(crate) restores: VecDeque<(crate::MobRestoreProposal, MobIdentity)>,
    pub(crate) events: VecDeque<MobEvent>,
    pub(crate) profiles: BTreeMap<PlayerId, MobPlayerProfile>,
}
impl Clone for MobRuntime {
    fn clone(&self) -> Self {
        Self {
            live: false,
            world: self.world,
            dimension: self.dimension,
            now: self.now,
            peaceful: self.peaceful,
            shared_spawn: self.shared_spawn,
            rules: self.rules.clone(),
            controller: None,
            stores: self.stores.clone(),
            active: self.active.clone(),
            pending: self.pending.clone(),
            restores: self.restores.clone(),
            events: VecDeque::new(),
            profiles: self.profiles.clone(),
        }
    }
}
impl MobRuntime {
    pub(crate) fn check_live(&self) -> Result<(), MobRuntimeError> {
        if self.live && self.controller.is_some() {
            Ok(())
        } else {
            Err(MobRuntimeError::OwnershipRequired)
        }
    }
    pub(crate) fn uuid_used(&self, uuid: EntityUuid) -> bool {
        self.restores
            .iter()
            .any(|(proposal, _)| proposal.key().uuid() == uuid)
            || self.pending.iter().any(|spawn| spawn.identity.uuid == uuid)
            || self
                .stores
                .values()
                .any(|store| store.handle_for_uuid(uuid).is_some())
    }
    pub(crate) fn identity_used(&self, uuid: EntityUuid, wire: EntityId) -> bool {
        self.restores
            .iter()
            .any(|(proposal, identity)| proposal.key().uuid() == uuid || identity.wire == wire)
            || self
                .pending
                .iter()
                .any(|entry| entry.identity.uuid == uuid || entry.identity.wire == wire)
            || self.stores.values().any(|store| {
                store.handle_for_uuid(uuid).is_some() || store.handle_for_wire_id(wire).is_some()
            })
            || self.profiles.values().any(|profile| profile.wire == wire)
    }
    pub(crate) fn snapshot(&self, wire: EntityId) -> Option<MobSnapshot> {
        let handle = self.active.get(&wire)?;
        let base = *self.stores.get(&handle.owner())?.get(handle).ok()?;
        let fact = self.controller.as_ref()?.health(wire)?;
        Some(MobSnapshot {
            base,
            kind: fact.kind(),
            health: fact.health(),
        })
    }
}
