//! Exact bounded native mob snapshots; no runtime wire identity or lease persists.

use crate::mobs::MobSavedState;
use crate::{EntityData, EntityTransform, MobIdentity, MobKind, MobRules, MobRuntimeError};
use ferrumc_core::{DimensionId, EntityUuid, WorldId};
use ferrumc_math::Vec3;
use ferrumc_storage::{EntityRecord, PersistentEntityKey, PersistentEntityRecord, SchemaVersion};

/// Fixed complete FCMB schema1 payload, independent of item FCIE and player data.
pub const NATIVE_MOB_ENTITY_BYTES: usize = 95;

/// Fully validated saved mob proposal. This contains no active identity/health
/// authority and can only become live through the genuine shard tick consumer.
#[derive(Debug, Clone, Copy)]
pub struct MobRestoreProposal {
    pub(crate) key: PersistentEntityKey,
    pub(crate) transform: EntityTransform,
    pub(crate) state: MobSavedState,
}
impl MobRestoreProposal {
    /// Exact saved persistent scope and UUID.
    pub const fn key(self) -> PersistentEntityKey {
        self.key
    }
    /// Original persistent UUID, without a runtime wire ID or handle.
    pub const fn uuid(self) -> EntityUuid {
        self.key.uuid()
    }
    /// Exact saved canonical transform, used by App to acquire resident tickets.
    pub const fn transform(self) -> EntityTransform {
        self.transform
    }
    /// Saved supported adult form.
    pub const fn kind(self) -> MobKind {
        self.state.kind
    }
    /// Exact saved positive authoritative health.
    pub const fn health(self) -> f32 {
        self.state.health
    }
}

/// Refused restore admission retaining the complete original validated proposal
/// and separately allocated fresh wire identity for retry or explicit retirement.
#[derive(Debug)]
pub struct MobRestoreRefusal {
    pub(crate) reason: MobRuntimeError,
    pub(crate) proposal: MobRestoreProposal,
    pub(crate) identity: MobIdentity,
}
impl MobRestoreRefusal {
    /// Classified precommit refusal.
    pub const fn reason(&self) -> &MobRuntimeError {
        &self.reason
    }
    /// Returns all original restore inputs without mutation.
    pub fn into_parts(self) -> (MobRuntimeError, MobRestoreProposal, MobIdentity) {
        (self.reason, self.proposal, self.identity)
    }
}

pub(crate) fn encode_mob_entity(
    base: EntityData,
    state: MobSavedState,
    rules: &MobRules,
) -> Result<(PersistentEntityKey, PersistentEntityRecord), MobRuntimeError> {
    state.validate(rules)?;
    let key = PersistentEntityKey::new(base.owner().world(), base.owner().dimension(), base.uuid());
    let mut body = Vec::with_capacity(NATIVE_MOB_ENTITY_BYTES);
    body.extend_from_slice(b"FCMB");
    body.extend_from_slice(&1_u16.to_be_bytes());
    body.extend_from_slice(&base.entity_type().protocol_id().to_be_bytes());
    body.extend_from_slice(&key.world().get().to_be_bytes());
    body.extend_from_slice(&key.dimension().get().to_be_bytes());
    body.extend_from_slice(&key.uuid().to_bytes());
    let pos = base.transform().position();
    for val in [pos.x, pos.y, pos.z] {
        body.extend_from_slice(&val.to_bits().to_be_bytes());
    }
    for val in [base.transform().yaw(), base.transform().pitch()] {
        body.extend_from_slice(&val.to_bits().to_be_bytes());
    }
    body.push(kind_byte(state.kind));
    body.extend_from_slice(&state.health.to_bits().to_be_bytes());
    body.extend_from_slice(&state.loot_rng.to_be_bytes());
    body.extend_from_slice(&state.brain_rng.to_be_bytes());
    body.extend_from_slice(&[
        state.direction,
        state.wander_remaining,
        state.attack_remaining,
        state.hurt_remaining,
    ]);
    body.extend_from_slice(&state.last_hit_amount.to_bits().to_be_bytes());
    let record = PersistentEntityRecord::new(
        base.chunk(),
        EntityRecord::new(SchemaVersion::new(1), body)?,
    )?;
    // Revalidate the complete encoded identity/transform relationship before App
    // receives a save record. No dirty state or runtime authority clears here.
    decode_with_rules(key, &record, rules)?;
    Ok((key, record))
}

/// Strictly decodes FCMB schema1. Unsupported/truncated/trailing bodies, invalid
/// health/timers, noncanonical transforms and outer scope/UUID/chunk disagreement
/// refuse before any registration or health admission; no defaults are supplied.
pub fn decode_mob_entity(
    key: PersistentEntityKey,
    record: &PersistentEntityRecord,
) -> Result<MobRestoreProposal, MobRuntimeError> {
    decode_with_rules(key, record, &MobRules::builtin()?)
}

fn decode_with_rules(
    key: PersistentEntityKey,
    record: &PersistentEntityRecord,
    rules: &MobRules,
) -> Result<MobRestoreProposal, MobRuntimeError> {
    if record.record().schema_version() != SchemaVersion::new(1) {
        return Err(MobRuntimeError::NativeRecord("payload schema"));
    }
    let bytes = record.record().data();
    if bytes.len() != NATIVE_MOB_ENTITY_BYTES {
        return Err(MobRuntimeError::NativeRecord("body length"));
    }
    let mut reader = Reader { bytes, at: 0 };
    if reader.take::<4>() != *b"FCMB" {
        return Err(MobRuntimeError::NativeRecord("magic"));
    }
    if u16::from_be_bytes(reader.take()) != 1 {
        return Err(MobRuntimeError::NativeRecord("native version"));
    }
    let entity_type = i32::from_be_bytes(reader.take());
    let world = WorldId::new(u32::from_be_bytes(reader.take()));
    let dimension = DimensionId::new(u32::from_be_bytes(reader.take()));
    let uuid = EntityUuid::from_bytes(reader.take());
    if PersistentEntityKey::new(world, dimension, uuid) != key {
        return Err(MobRuntimeError::NativeRecord("outer key"));
    }
    let pos = Vec3::new(reader.f64(), reader.f64(), reader.f64());
    let yaw = reader.f32();
    let pitch = reader.f32();
    let transform = EntityTransform::try_new(pos, yaw, pitch)?;
    if yaw.to_bits() != transform.yaw().to_bits() || pitch.to_bits() != transform.pitch().to_bits()
    {
        return Err(MobRuntimeError::NativeRecord("noncanonical rotation"));
    }
    if transform.chunk() != record.chunk() {
        return Err(MobRuntimeError::NativeRecord("outer chunk"));
    }
    let kind = match reader.take::<1>()[0] {
        0 => MobKind::Sheep,
        1 => MobKind::Cow,
        2 => MobKind::Pig,
        3 => MobKind::Chicken,
        4 => MobKind::Zombie,
        _ => return Err(MobRuntimeError::NativeRecord("mob kind")),
    };
    if rules.attributes(kind).entity_type().protocol_id() != entity_type {
        return Err(MobRuntimeError::NativeRecord("entity type"));
    }
    let health = reader.f32();
    let loot_rng = u64::from_be_bytes(reader.take());
    let brain_rng = u64::from_be_bytes(reader.take());
    let [direction, wander_remaining, attack_remaining, hurt_remaining] = reader.take();
    let state = MobSavedState {
        kind,
        health,
        loot_rng,
        brain_rng,
        direction,
        wander_remaining,
        attack_remaining,
        hurt_remaining,
        last_hit_amount: reader.f32(),
    };
    state.validate(rules)?;
    Ok(MobRestoreProposal {
        key,
        transform,
        state,
    })
}

const fn kind_byte(kind: MobKind) -> u8 {
    match kind {
        MobKind::Sheep => 0,
        MobKind::Cow => 1,
        MobKind::Pig => 2,
        MobKind::Chicken => 3,
        MobKind::Zombie => 4,
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> [u8; N] {
        let mut result = [0; N];
        result.copy_from_slice(&self.bytes[self.at..self.at + N]);
        self.at += N;
        result
    }
    fn f32(&mut self) -> f32 {
        f32::from_bits(u32::from_be_bytes(self.take()))
    }
    fn f64(&mut self) -> f64 {
        f64::from_bits(u64::from_be_bytes(self.take()))
    }
}

pub(crate) type MobRecord = (PersistentEntityKey, PersistentEntityRecord);
