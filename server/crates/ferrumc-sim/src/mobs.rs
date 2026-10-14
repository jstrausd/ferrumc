//! Bounded adult ground-mob subset, with canonical transforms owned by Sim.
//!
//! Exact pinned JVM dimensions, attributes, metadata and static collision boxes
//! support sheep, cow, pig, chicken and zombie. Walking, wandering and targeting
//! are deterministic native rules, not full vanilla goal/navigation parity.
//! No chunk load, teleport, item publication or player-health mutation occurs here.
//! A death is prepared first; the owner reserves drop/output capacity before
//! applying the owned plan. Refusal returns that same plan, retaining its items.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Arc, Weak};

use ferrumc_core::EntityId;
use ferrumc_items::{ComponentPatch, ItemId, ItemStack};
use ferrumc_math::{BlockPos, Vec3};
use ferrumc_registry::entity_type::EntityTypeId;
use ferrumc_world::BlockStateId;

#[path = "mobs/facts.rs"]
mod facts;
#[path = "mobs/physics.rs"]
mod physics;

/// Hard active-mob and per-step actor/target limit.
pub const MAX_ACTIVE_MOBS: usize = 64;
/// Hard resident block-reader calls per actor (cached, including sight checks).
pub const MAX_MOB_READS: usize = 512;
/// Hard collision-box work per actor.
pub const MAX_MOB_BOX_WORK: usize = 8192;

/// Adult forms supported by this controller. Sheep are white and unsheared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MobKind {
    /// Adult white unsheared sheep.
    Sheep,
    /// Adult cow.
    Cow,
    /// Adult unsaddled pig.
    Pig,
    /// Adult ordinary chicken, excluding jockeys.
    Chicken,
    /// Adult unequipped zombie.
    Zombie,
}
impl MobKind {
    const ALL: [Self; 5] = [
        Self::Sheep,
        Self::Cow,
        Self::Pig,
        Self::Chicken,
        Self::Zombie,
    ];
    const fn index(self) -> usize {
        match self {
            Self::Sheep => 0,
            Self::Cow => 1,
            Self::Pig => 2,
            Self::Chicken => 3,
            Self::Zombie => 4,
        }
    }
    /// Canonical namespaced entity type name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sheep => "minecraft:sheep",
            Self::Cow => "minecraft:cow",
            Self::Pig => "minecraft:pig",
            Self::Chicken => "minecraft:chicken",
            Self::Zombie => "minecraft:zombie",
        }
    }
}

/// Classified refusals. Terrain refusal never initiates a load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobError {
    /// Corrupt or mismatched pinned executable facts.
    InvalidFacts,
    /// Finite coordinate, health, light, probability or identity input failed validation.
    InvalidInput,
    /// Active, actor or target limit reached.
    Capacity,
    /// Duplicate admission or duplicated actor/target input.
    DuplicateEntity,
    /// Mob is not admitted to this controller.
    UnknownEntity,
    /// Tick is not the exact successor of the previous step, or time overflowed.
    InvalidTick,
    /// Collision/read work hit its fixed budget.
    WorkLimit,
    /// Required block is unavailable to the resident reader.
    IncompleteTerrain,
    /// State is unknown, dynamic or outside the admitted static shape subset.
    UnsupportedState,
    /// Spawn light, distance, difficulty, floor or obstruction check refused.
    SpawnRefused,
    /// Already dead mob cannot receive another damage or death grant.
    Dead,
    /// Damage is no greater than the preceding hit in the protected half-window.
    HurtCooldown,
    /// Stale damage plan, including removed/reused ids or expired damage time.
    StalePlan,
    /// Plan belongs to another controller incarnation.
    ForeignPlan,
    /// Dirty version/admission identity cannot advance without wrapping.
    VersionExhausted,
}
impl fmt::Display for MobError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "mob subset refusal: {self:?}")
    }
}
impl std::error::Error for MobError {}

/// Original adult dimensions and base attributes; no runtime attribute modifiers.
#[derive(Debug, Clone, Copy)]
pub struct MobAttributes {
    entity_type: EntityTypeId,
    width: f64,
    height: f64,
    eye_height: f64,
    max_health: f32,
    movement_speed: f64,
    follow_range: f64,
    attack_damage: f32,
    armor: f32,
    step_height: f64,
}
impl MobAttributes {
    /// Genuine protocol-bound entity type.
    pub const fn entity_type(self) -> EntityTypeId {
        self.entity_type
    }
    /// Original adult width.
    pub const fn width(self) -> f64 {
        self.width
    }
    /// Original adult height.
    pub const fn height(self) -> f64 {
        self.height
    }
    /// Original adult eye height.
    pub const fn eye_height(self) -> f64 {
        self.eye_height
    }
    /// Original base maximum health.
    pub const fn max_health(self) -> f32 {
        self.max_health
    }
    /// Original base movement-speed attribute, before native walking policy.
    pub const fn movement_speed(self) -> f64 {
        self.movement_speed
    }
    /// Original base follow range.
    pub const fn follow_range(self) -> f64 {
        self.follow_range
    }
    /// Original base attack damage (zero for this passive subset).
    pub const fn attack_damage(self) -> f32 {
        self.attack_damage
    }
    /// Original base armor.
    pub const fn armor(self) -> f32 {
        self.armor
    }
    /// Original base step-height attribute.
    pub const fn step_height(self) -> f64 {
        self.step_height
    }
}

/// One genuine metadata accessor index and serializer id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobMetadataAccessor {
    index: u8,
    serializer: i32,
}
impl MobMetadataAccessor {
    /// Original accessor index.
    pub const fn index(self) -> u8 {
        self.index
    }
    /// Original registry serializer id.
    pub const fn serializer(self) -> i32 {
        self.serializer
    }
}

/// Immutable pinned rules shared across bounded controllers.
#[derive(Debug, Clone)]
pub struct MobRules {
    facts: Arc<facts::MobFacts>,
}
impl MobRules {
    /// Validate embedded executable facts and bind native registry type ids.
    pub fn builtin() -> Result<Self, MobError> {
        Ok(Self {
            facts: facts::builtin()?,
        })
    }
    /// Original adult dimensions and base attributes.
    pub fn attributes(&self, kind: MobKind) -> MobAttributes {
        self.facts.attributes[kind.index()]
    }
    /// Genuine inherited health metadata accessor; initial value is max health.
    pub fn health_metadata(&self) -> MobMetadataAccessor {
        self.facts.metadata[0]
    }
    /// Genuine inherited mob flags accessor; normal adult initial byte is zero.
    pub fn flags_metadata(&self) -> MobMetadataAccessor {
        self.facts.metadata[1]
    }
    /// Genuine baby accessor; supported initial value is false.
    pub fn baby_metadata(&self, kind: MobKind) -> MobMetadataAccessor {
        self.facts.metadata[if kind == MobKind::Zombie { 5 } else { 2 }]
    }
    /// White unsheared sheep wool accessor; supported initial byte is zero.
    pub fn sheep_wool_metadata(&self) -> MobMetadataAccessor {
        self.facts.metadata[3]
    }
    /// Pig boost-time accessor; initial integer is zero.
    pub fn pig_boost_metadata(&self) -> MobMetadataAccessor {
        self.facts.metadata[4]
    }
    /// Zombie special-type accessor; initial integer is zero.
    pub fn zombie_special_metadata(&self) -> MobMetadataAccessor {
        self.facts.metadata[6]
    }
    /// Zombie drowned-conversion accessor; initial boolean is false.
    pub fn zombie_conversion_metadata(&self) -> MobMetadataAccessor {
        self.facts.metadata[7]
    }
    /// Original horizontal attack-box inflation for an unmounted zombie.
    pub fn attack_reach(&self) -> f64 {
        self.facts.attack_reach
    }
    /// Validate one ordinary, dry, resident Overworld spawn without loading.
    ///
    /// Caller owns border/shared-spawn/biome/category/population checks. This
    /// subset deliberately caps all spawns at 128 blocks from a player.
    pub fn validate_spawn(
        &self,
        kind: MobKind,
        position: Vec3,
        context: MobSpawnContext,
        mut read: impl FnMut(BlockPos) -> Option<BlockStateId>,
    ) -> Result<(), MobError> {
        validate_position(position)?;
        if context.nearest_player_distance_squared <= 576.0
            || context.nearest_player_distance_squared > 16384.0
        {
            return Err(MobError::SpawnRefused);
        }
        if kind == MobKind::Zombie {
            if context.peaceful
                || context.light.sky > context.sky_roll
                || context.light.block != 0
                || context.light.raw > context.dark_roll
            {
                return Err(MobError::SpawnRefused);
            }
        } else if context.light.raw <= 8 {
            return Err(MobError::SpawnRefused);
        }
        let floor = BlockPos::new(
            position.x.floor() as i32,
            position.y.floor() as i32 - 1,
            position.z.floor() as i32,
        );
        let mut terrain = physics::Terrain::new(&self.facts, &mut read);
        let support = terrain.state(floor)?;
        if support.flags & (1 << (3 + kind.index())) == 0
            || (kind != MobKind::Zombie && support.flags & 4 == 0)
        {
            return Err(MobError::SpawnRefused);
        }
        for y in [floor.y() + 1, floor.y() + 2] {
            if terrain.state(BlockPos::new(floor.x(), y, floor.z()))?.flags
                & (1 << (8 + kind.index()))
                == 0
            {
                return Err(MobError::SpawnRefused);
            }
        }
        if !terrain.clear_body(position, self.attributes(kind))?
            || !terrain.supported(position, self.attributes(kind))?
        {
            return Err(MobError::SpawnRefused);
        }
        Ok(())
    }
}

/// Verified resident sky, block and raw brightness at a spawn position.
#[derive(Debug, Clone, Copy)]
pub struct MobSpawnLight {
    sky: u8,
    block: u8,
    raw: u8,
}
impl MobSpawnLight {
    /// Values must all be in 0..=15. Raw brightness includes sky darkening.
    pub fn new(sky: u8, block: u8, raw: u8) -> Result<Self, MobError> {
        if sky > 15 || block > 15 || raw > 15 {
            return Err(MobError::InvalidInput);
        }
        Ok(Self { sky, block, raw })
    }
}
/// Owner-supplied ordinary Overworld spawn facts and already-drawn light rolls.
#[derive(Debug, Clone, Copy)]
pub struct MobSpawnContext {
    light: MobSpawnLight,
    nearest_player_distance_squared: f64,
    peaceful: bool,
    sky_roll: u8,
    dark_roll: u8,
}
impl MobSpawnContext {
    /// Sky roll is uniform 0..32 exclusive; Overworld dark roll is 0..8 exclusive.
    /// No available player must be represented by refusing before this call.
    pub fn new(
        light: MobSpawnLight,
        nearest_player_distance_squared: f64,
        peaceful: bool,
        sky_roll: u8,
        dark_roll: u8,
    ) -> Result<Self, MobError> {
        if !nearest_player_distance_squared.is_finite()
            || nearest_player_distance_squared < 0.0
            || sky_roll > 31
            || dark_roll > 7
        {
            return Err(MobError::InvalidInput);
        }
        Ok(Self {
            light,
            nearest_player_distance_squared,
            peaceful,
            sky_roll,
            dark_roll,
        })
    }
}

/// Borrowed tick snapshot of a canonical entity transform; never stored here.
#[derive(Debug, Clone, Copy)]
pub struct MobActor {
    id: EntityId,
    position: Vec3,
    yaw: f32,
}
impl MobActor {
    /// Validate a canonical owner snapshot. Identity must be positive.
    pub fn new(id: EntityId, position: Vec3, yaw: f32) -> Result<Self, MobError> {
        validate_position(position)?;
        if id.get() <= 0 || !yaw.is_finite() {
            return Err(MobError::InvalidInput);
        }
        Ok(Self {
            id,
            position,
            yaw: yaw.rem_euclid(360.0),
        })
    }
    /// Canonical wire entity identity.
    pub const fn id(self) -> EntityId {
        self.id
    }
    /// Canonical feet position supplied for this tick.
    pub const fn position(self) -> Vec3 {
        self.position
    }
    /// Validated owner-supplied facing yaw.
    pub const fn yaw(self) -> f32 {
        self.yaw
    }
}
/// Borrowed standing-player target snapshot.
#[derive(Debug, Clone, Copy)]
pub struct MobTarget {
    id: EntityId,
    position: Vec3,
    targetable: bool,
}
impl MobTarget {
    /// Caller filters live eligible Survival/adventure players and supplies
    /// false for creative, spectator, dead, disconnected or non-standing poses.
    pub fn new(id: EntityId, position: Vec3, targetable: bool) -> Result<Self, MobError> {
        validate_position(position)?;
        if id.get() <= 0 {
            return Err(MobError::InvalidInput);
        }
        Ok(Self {
            id,
            position,
            targetable,
        })
    }
    /// Target wire entity identity.
    pub const fn id(self) -> EntityId {
        self.id
    }
}
/// Safe local grounded movement intention. Canonical owner may refuse it.
#[derive(Debug, Clone, Copy)]
pub struct MobMove {
    id: EntityId,
    before: Vec3,
    after: Vec3,
    yaw: f32,
}
impl MobMove {
    /// Mob identity.
    pub const fn id(self) -> EntityId {
        self.id
    }
    /// Required current canonical feet position; owner rejects stale snapshots.
    pub const fn before(self) -> Vec3 {
        self.before
    }
    /// Candidate canonical feet position, at most one native walking step away.
    pub const fn after(self) -> Vec3 {
        self.after
    }
    /// Candidate facing yaw in degrees.
    pub const fn yaw(self) -> f32 {
        self.yaw
    }
}
/// Eligible zombie attack intention; player vitals owner resolves the hit.
#[derive(Debug, Clone, Copy)]
pub struct MobAttack {
    attacker: EntityId,
    target: EntityId,
    base_damage: f32,
}
impl MobAttack {
    /// Admitted attacker identity.
    pub const fn attacker(self) -> EntityId {
        self.attacker
    }
    /// Eligible target identity.
    pub const fn target(self) -> EntityId {
        self.target
    }
    /// Original unequipped zombie base damage, before difficulty/armor.
    pub const fn base_damage(self) -> f32 {
        self.base_damage
    }
}
/// Bounded per-tick intentions. Brain advancement never grants health or items.
#[derive(Debug)]
pub struct MobStep {
    movements: Vec<MobMove>,
    attacks: Vec<MobAttack>,
    blocked: usize,
}
impl MobStep {
    /// At most 64 local grounded movement intentions.
    pub fn movements(&self) -> &[MobMove] {
        &self.movements
    }
    /// At most 64 eligible zombie attack intentions.
    pub fn attacks(&self) -> &[MobAttack] {
        &self.attacks
    }
    /// Actors blocked by unavailable/unsupported terrain or the local work cap.
    pub const fn blocked(&self) -> usize {
        self.blocked
    }
}

#[derive(Debug, Clone, Copy)]
struct Brain {
    rng: u64,
    direction: u8,
    wander_until: u64,
    next_attack: u64,
}
#[derive(Debug)]
struct Entry {
    kind: MobKind,
    admission: u64,
    health: f32,
    generation: u64,
    dirty: bool,
    loot_rng: u64,
    brain: Brain,
    protected_until: u64,
    last_hit_amount: f32,
}

/// A persistence/metadata fact. Acknowledgement must match admission and version.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MobHealthFact {
    id: EntityId,
    kind: MobKind,
    admission: u64,
    health: f32,
    generation: u64,
}
impl MobHealthFact {
    /// Mob identity.
    pub const fn id(self) -> EntityId {
        self.id
    }
    /// Supported adult form.
    pub const fn kind(self) -> MobKind {
        self.kind
    }
    /// Non-reused admission serial within this controller.
    pub const fn admission(self) -> u64 {
        self.admission
    }
    /// Exact current authoritative health.
    pub const fn health(self) -> f32 {
        self.health
    }
    /// Monotonic health generation.
    pub const fn generation(self) -> u64 {
        self.generation
    }
}
fn health_fact(id: EntityId, entry: &Entry) -> MobHealthFact {
    MobHealthFact {
        id,
        kind: entry.kind,
        admission: entry.admission,
        health: entry.health,
        generation: entry.generation,
    }
}

// Fixed controller state without runtime wire identity, admission or incarnation.
// Relative timers preserve logical durations across a fresh process clock.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MobSavedState {
    pub(crate) kind: MobKind,
    pub(crate) health: f32,
    pub(crate) loot_rng: u64,
    pub(crate) brain_rng: u64,
    pub(crate) direction: u8,
    pub(crate) wander_remaining: u8,
    pub(crate) attack_remaining: u8,
    pub(crate) hurt_remaining: u8,
    pub(crate) last_hit_amount: f32,
}
impl MobSavedState {
    pub(crate) fn validate(self, rules: &MobRules) -> Result<(), MobError> {
        if !self.health.is_finite()
            || self.health <= 0.0
            || self.health > rules.attributes(self.kind).max_health()
            || !self.last_hit_amount.is_finite()
            || self.last_hit_amount < 0.0
            || self.direction > 8
            || self.wander_remaining > 40
            || self.attack_remaining > 20
            || self.hurt_remaining > 10
        {
            return Err(MobError::InvalidInput);
        }
        Ok(())
    }
}

/// Sole owner of bounded brains and health, never canonical entity transforms.
#[derive(Debug)]
pub struct MobController {
    rules: MobRules,
    incarnation: Arc<()>,
    entries: BTreeMap<EntityId, Entry>,
    next_admission: u64,
    last_tick: Option<u64>,
}
impl MobController {
    /// Empty controller with shared validated pinned rules.
    pub fn new(rules: MobRules) -> Self {
        Self {
            rules,
            incarnation: Arc::new(()),
            entries: BTreeMap::new(),
            next_admission: 1,
            last_tick: None,
        }
    }
    /// Number of retained active/dead admissions, at most 64.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    /// Whether no mob admissions remain.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Admit an already-created canonical adult mob. Owner validates spawn first.
    /// Admission returns the retained initial dirty health fact.
    pub fn admit(
        &mut self,
        id: EntityId,
        kind: MobKind,
        seed: u64,
    ) -> Result<MobHealthFact, MobError> {
        if id.get() <= 0 {
            return Err(MobError::InvalidInput);
        }
        if self.entries.contains_key(&id) {
            return Err(MobError::DuplicateEntity);
        }
        if self.entries.len() >= MAX_ACTIVE_MOBS {
            return Err(MobError::Capacity);
        }
        let next = self
            .next_admission
            .checked_add(1)
            .ok_or(MobError::VersionExhausted)?;
        let entry = Entry {
            kind,
            admission: self.next_admission,
            health: self.rules.attributes(kind).max_health,
            generation: 1,
            dirty: true,
            loot_rng: seed ^ 0xBADC_0FFE_E0DD_F00D,
            brain: Brain {
                rng: seed,
                direction: 0,
                wander_until: 0,
                next_attack: 0,
            },
            protected_until: 0,
            last_hit_amount: 0.0,
        };
        let fact = health_fact(id, &entry);
        self.entries.insert(id, entry);
        self.next_admission = next;
        Ok(fact)
    }
    pub(crate) fn capture_state(&self, id: EntityId, now: u64) -> Result<MobSavedState, MobError> {
        if self.last_tick.is_some_and(|last| now < last) {
            return Err(MobError::InvalidTick);
        }
        let entry = self.entries.get(&id).ok_or(MobError::UnknownEntity)?;
        let state = MobSavedState {
            kind: entry.kind,
            health: entry.health,
            loot_rng: entry.loot_rng,
            brain_rng: entry.brain.rng,
            direction: entry.brain.direction,
            wander_remaining: u8::try_from(entry.brain.wander_until.saturating_sub(now))
                .map_err(|_| MobError::InvalidTick)?,
            attack_remaining: u8::try_from(entry.brain.next_attack.saturating_sub(now))
                .map_err(|_| MobError::InvalidTick)?,
            hurt_remaining: u8::try_from(entry.protected_until.saturating_sub(now))
                .map_err(|_| MobError::InvalidTick)?,
            last_hit_amount: entry.last_hit_amount,
        };
        state.validate(&self.rules)?;
        Ok(state)
    }

    pub(crate) fn restore_state(
        &mut self,
        id: EntityId,
        now: u64,
        state: MobSavedState,
    ) -> Result<(), MobError> {
        state.validate(&self.rules)?;
        if now > u64::MAX - 40 {
            return Err(MobError::InvalidTick);
        }
        self.admit(id, state.kind, 0)?;
        let Some(entry) = self.entries.get_mut(&id) else {
            unreachable!("fresh restored admission");
        };
        entry.health = state.health;
        entry.loot_rng = state.loot_rng;
        entry.brain = Brain {
            rng: state.brain_rng,
            direction: state.direction,
            wander_until: now + u64::from(state.wander_remaining),
            next_attack: now + u64::from(state.attack_remaining),
        };
        entry.protected_until = now + u64::from(state.hurt_remaining);
        entry.last_hit_amount = state.last_hit_amount;
        Ok(())
    }

    /// Current exact authoritative health/persistence fact.
    pub fn health(&self, id: EntityId) -> Option<MobHealthFact> {
        self.entries.get(&id).map(|entry| health_fact(id, entry))
    }
    /// Retained dirty facts, at most 64, until an exact acknowledgement.
    pub fn dirty_health(&self) -> impl Iterator<Item = MobHealthFact> + '_ {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.dirty)
            .map(|(&id, entry)| health_fact(id, entry))
    }
    /// Clear dirty only if the completed save represents this exact admission/version.
    pub fn ack_health_saved(&mut self, fact: MobHealthFact) -> bool {
        if let Some(entry) = self.entries.get_mut(&fact.id) {
            if entry.admission == fact.admission
                && entry.generation == fact.generation
                && entry.health.to_bits() == fact.health.to_bits()
            {
                entry.dirty = false;
                return true;
            }
        }
        false
    }
    /// Remove an admission and return its final fact for owner retention/persistence.
    /// Outstanding plans become stale. This operation does not persist the fact.
    pub fn remove(&mut self, id: EntityId) -> Option<MobHealthFact> {
        self.entries
            .remove(&id)
            .map(|entry| health_fact(id, &entry))
    }
    /// Advance deterministic bounded local AI by one exact successor tick.
    ///
    /// Preflight refusal mutates nothing. Each actor gets a 512-call cached
    /// resident reader and 8192-box work budget. Missing/unsupported cells close
    /// movement and sight; they never create loads or teleports. Movement owner
    /// may refuse intentions without losing health/drop ownership.
    pub fn step(
        &mut self,
        now: u64,
        actors: &[MobActor],
        targets: &[MobTarget],
        mut read: impl FnMut(BlockPos) -> Option<BlockStateId>,
    ) -> Result<MobStep, MobError> {
        if self
            .last_tick
            .is_some_and(|last| last.checked_add(1) != Some(now))
            || now > u64::MAX - 40
        {
            return Err(MobError::InvalidTick);
        }
        if actors.len() > MAX_ACTIVE_MOBS || targets.len() > MAX_ACTIVE_MOBS {
            return Err(MobError::Capacity);
        }
        let mut seen = BTreeSet::new();
        for actor in actors {
            if !seen.insert(actor.id) {
                return Err(MobError::DuplicateEntity);
            }
            if !self.entries.contains_key(&actor.id) {
                return Err(MobError::UnknownEntity);
            }
        }
        seen.clear();
        for target in targets {
            if !seen.insert(target.id) {
                return Err(MobError::DuplicateEntity);
            }
            if self.entries.contains_key(&target.id) {
                return Err(MobError::InvalidInput);
            }
        }
        let mut brains = Vec::with_capacity(actors.len());
        let mut output = MobStep {
            movements: Vec::with_capacity(actors.len()),
            attacks: Vec::new(),
            blocked: 0,
        };
        for actor in actors {
            let Some(entry) = self.entries.get(&actor.id) else {
                return Err(MobError::UnknownEntity);
            };
            if entry.health <= 0.0 {
                continue;
            }
            let brain = self.plan_actor(now, actor, entry, targets, &mut read, &mut output);
            brains.push((actor.id, brain));
        }
        for (id, brain) in brains {
            if let Some(entry) = self.entries.get_mut(&id) {
                entry.brain = brain;
            }
        }
        self.last_tick = Some(now);
        Ok(output)
    }
    fn plan_actor(
        &self,
        now: u64,
        actor: &MobActor,
        entry: &Entry,
        targets: &[MobTarget],
        read: &mut impl FnMut(BlockPos) -> Option<BlockStateId>,
        output: &mut MobStep,
    ) -> Brain {
        let mut brain = entry.brain;
        let attributes = self.rules.attributes(entry.kind);
        let mut terrain = physics::Terrain::new(&self.rules.facts, read);
        if !matches!(terrain.clear_body(actor.position, attributes), Ok(true))
            || !matches!(terrain.supported(actor.position, attributes), Ok(true))
        {
            output.blocked += 1;
            return brain;
        }
        let target = if entry.kind == MobKind::Zombie {
            targets
                .iter()
                .filter(|target| target.targetable)
                .filter_map(|target| {
                    let distance = (target.position - actor.position).length_squared();
                    (distance <= attributes.follow_range * attributes.follow_range)
                        .then_some((distance, target))
                })
                .min_by(|(a, ta), (b, tb)| a.total_cmp(b).then_with(|| ta.id.cmp(&tb.id)))
                .map(|(_, target)| target)
        } else {
            None
        };
        let mut heading = None;
        if let Some(target) = target {
            match terrain.line_of_sight(
                actor.position + Vec3::new(0.0, attributes.eye_height, 0.0),
                target.position + Vec3::new(0.0, f64::from(1.62_f32), 0.0),
            ) {
                Ok(true) => {
                    if physics::in_attack_range(
                        actor.position,
                        attributes,
                        target.position,
                        self.rules.facts.attack_reach,
                    ) {
                        if now >= brain.next_attack {
                            output.attacks.push(MobAttack {
                                attacker: actor.id,
                                target: target.id,
                                base_damage: attributes.attack_damage,
                            });
                            brain.next_attack = now + 20;
                        }
                    } else {
                        heading = horizontal_direction(target.position - actor.position);
                    }
                }
                Ok(false) => {}
                Err(_) => {
                    output.blocked += 1;
                }
            }
        } else {
            if now >= brain.wander_until {
                brain.direction = draw(&mut brain.rng, 9) as u8;
                brain.wander_until = now + 40;
            }
            if brain.direction < 8 {
                let (x, z) = DIRECTIONS[usize::from(brain.direction)];
                heading = horizontal_direction(Vec3::new(x, 0.0, z));
            }
        }
        if let Some(direction) = heading {
            match plan_move(*actor, direction, attributes, &mut terrain) {
                Ok(Some(movement)) => output.movements.push(movement),
                Ok(None) | Err(_) => output.blocked += 1,
            }
        }
        brain
    }
    /// Prepare ordinary direct damage without changing health, loot RNG or dirtiness.
    ///
    /// Uses original unenchanted armor and the normal 20-tick hurt window: in
    /// its first half only the positive increase above the previous raw hit is
    /// applied. No enchantments, effects, equipment, shields or knockback.
    pub fn prepare_damage(
        &self,
        now: u64,
        victim: EntityId,
        raw_damage: f32,
        context: MobDamageContext,
    ) -> Result<MobDamagePlan, MobError> {
        if !raw_damage.is_finite() || raw_damage <= 0.0 {
            return Err(MobError::InvalidInput);
        }
        if self.last_tick.is_some_and(|last| now < last) || now > u64::MAX - 20 {
            return Err(MobError::InvalidTick);
        }
        let entry = self.entries.get(&victim).ok_or(MobError::UnknownEntity)?;
        if entry.health <= 0.0 {
            return Err(MobError::Dead);
        }
        if now < entry.protected_until.saturating_sub(10) {
            return Err(MobError::InvalidTick);
        }
        let protected = now < entry.protected_until;
        let incoming = if protected {
            if raw_damage <= entry.last_hit_amount {
                return Err(MobError::HurtCooldown);
            }
            raw_damage - entry.last_hit_amount
        } else {
            raw_damage
        };
        let damage = ordinary_armor_damage(incoming, self.rules.attributes(entry.kind).armor);
        let health = (entry.health - damage).max(0.0);
        let generation = entry
            .generation
            .checked_add(1)
            .ok_or(MobError::VersionExhausted)?;
        let mut loot_rng = entry.loot_rng;
        let (drops, experience) = if health <= 0.0 {
            death_drops(entry.kind, context, &mut loot_rng)?
        } else {
            (Vec::new(), 0)
        };
        Ok(MobDamagePlan {
            incarnation: Arc::downgrade(&self.incarnation),
            before: health_fact(victim, entry),
            now,
            health,
            generation,
            raw_damage,
            protected_until: if protected {
                entry.protected_until
            } else {
                now + 10
            },
            loot_rng,
            drops,
            experience,
        })
    }
    /// Commit after the owner reserves every required item/output slot.
    /// A refusal retains and returns the same owned, unpublished plan.
    pub fn apply_damage(
        &mut self,
        plan: MobDamagePlan,
    ) -> Result<MobDamageReceipt, MobDamageRefusal> {
        let error = if !Weak::ptr_eq(&plan.incarnation, &Arc::downgrade(&self.incarnation)) {
            Some(MobError::ForeignPlan)
        } else if self.last_tick.is_some_and(|last| last > plan.now) {
            Some(MobError::StalePlan)
        } else {
            match self.entries.get(&plan.before.id) {
                Some(entry) if health_fact(plan.before.id, entry) == plan.before => None,
                _ => Some(MobError::StalePlan),
            }
        };
        if let Some(error) = error {
            return Err(MobDamageRefusal {
                error,
                plan: Box::new(plan),
            });
        }
        let Some(entry) = self.entries.get_mut(&plan.before.id) else {
            return Err(MobDamageRefusal {
                error: MobError::StalePlan,
                plan: Box::new(plan),
            });
        };
        entry.health = plan.health;
        entry.generation = plan.generation;
        entry.dirty = true;
        entry.loot_rng = plan.loot_rng;
        entry.protected_until = plan.protected_until;
        entry.last_hit_amount = plan.raw_damage;
        Ok(MobDamageReceipt {
            fact: health_fact(plan.before.id, entry),
            drops: plan.drops,
            experience: plan.experience,
        })
    }
}

/// Normal death-loot inputs. Owner supplies actual burning and player-kill credit.
#[derive(Debug, Clone, Copy)]
pub struct MobDamageContext {
    burning: bool,
    killed_by_player: bool,
}
impl MobDamageContext {
    /// No looting/smelts-loot enchantment, baby, sheared or equipment variant.
    pub const fn new(burning: bool, killed_by_player: bool) -> Self {
        Self {
            burning,
            killed_by_player,
        }
    }
}
/// Owned staged damage; intentionally neither Clone nor externally constructible.
#[derive(Debug)]
pub struct MobDamagePlan {
    incarnation: Weak<()>,
    before: MobHealthFact,
    now: u64,
    health: f32,
    generation: u64,
    raw_damage: f32,
    protected_until: u64,
    loot_rng: u64,
    drops: Vec<ItemStack>,
    experience: u8,
}
impl MobDamagePlan {
    /// Expected current health/version; preparation has not changed it.
    pub const fn before(&self) -> MobHealthFact {
        self.before
    }
    /// Exact resulting health after committing the plan.
    pub const fn health_after(&self) -> f32 {
        self.health
    }
    /// Whether this plan would commit death.
    pub fn is_death(&self) -> bool {
        self.health <= 0.0
    }
    /// At most two owned ordinary stacks; owner reserves this exact count.
    pub fn drops(&self) -> &[ItemStack] {
        &self.drops
    }
    /// Original normal XP range draw, zero without player-kill credit.
    pub const fn experience(&self) -> u8 {
        self.experience
    }
}
/// A refused plan remains owned, including every unpublished death item.
#[derive(Debug)]
pub struct MobDamageRefusal {
    error: MobError,
    plan: Box<MobDamagePlan>,
}
impl MobDamageRefusal {
    /// Classified refusal.
    pub const fn error(&self) -> MobError {
        self.error
    }
    /// Recover the exact unpublished owned plan.
    pub fn into_plan(self) -> MobDamagePlan {
        *self.plan
    }
}
/// Committed damage and its owned death drops. Intentionally non-Clone.
#[derive(Debug)]
pub struct MobDamageReceipt {
    fact: MobHealthFact,
    drops: Vec<ItemStack>,
    experience: u8,
}
impl MobDamageReceipt {
    /// Committed health/version to broadcast and persist.
    pub const fn fact(&self) -> MobHealthFact {
        self.fact
    }
    /// Borrowed committed drops retained until the owner can publish them.
    pub fn drops(&self) -> &[ItemStack] {
        &self.drops
    }
    /// Transfer committed drop ownership exactly once.
    pub fn into_drops(self) -> Vec<ItemStack> {
        self.drops
    }
    /// XP reward to consume before moving this receipt into drops.
    pub const fn experience(&self) -> u8 {
        self.experience
    }
}

fn plan_move<F: FnMut(BlockPos) -> Option<BlockStateId>>(
    actor: MobActor,
    direction: Vec3,
    attributes: MobAttributes,
    terrain: &mut physics::Terrain<'_, F>,
) -> Result<Option<MobMove>, MobError> {
    let distance = attributes.movement_speed * 0.5;
    let attempts = [
        direction,
        Vec3::new(direction.x, 0.0, 0.0),
        Vec3::new(0.0, 0.0, direction.z),
        Vec3::new(-direction.z, 0.0, direction.x),
        Vec3::new(direction.z, 0.0, -direction.x),
    ];
    for candidate in attempts {
        let Some(candidate) = horizontal_direction(candidate) else {
            continue;
        };
        let proposal = actor.position + candidate * distance;
        validate_position(proposal)?;
        if let Some(after) = terrain.grounded_move(actor.position, proposal, attributes)? {
            validate_position(after)?;
            let yaw = ((-candidate.x).atan2(candidate.z).to_degrees() as f32).rem_euclid(360.0);
            return Ok(Some(MobMove {
                id: actor.id,
                before: actor.position,
                after,
                yaw,
            }));
        }
    }
    Ok(None)
}
const DIRECTIONS: [(f64, f64); 8] = [
    (0.0, -1.0),
    (1.0, -1.0),
    (1.0, 0.0),
    (1.0, 1.0),
    (0.0, 1.0),
    (-1.0, 1.0),
    (-1.0, 0.0),
    (-1.0, -1.0),
];
fn horizontal_direction(vector: Vec3) -> Option<Vec3> {
    Vec3::new(vector.x, 0.0, vector.z).normalize()
}
fn validate_position(position: Vec3) -> Result<(), MobError> {
    if !position.x.is_finite()
        || !position.y.is_finite()
        || !position.z.is_finite()
        || position.x.abs() > 30_000_000.0
        || position.z.abs() > 30_000_000.0
        || position.y.abs() > 20_000_000.0
    {
        return Err(MobError::InvalidInput);
    }
    Ok(())
}
fn draw(rng: &mut u64, bound: u32) -> u32 {
    // Bounded multiply-high mapping, native SplitMix64 sequence, not vanilla loot RNG.
    *rng = rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = *rng;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    ((u128::from(value) * u128::from(bound)) >> 64) as u32
}
fn ordinary_armor_damage(damage: f32, armor: f32) -> f32 {
    let effective = (armor - damage / 2.0).clamp(armor * 0.2, 20.0);
    damage * (1.0 - effective / 25.0)
}
fn death_drops(
    kind: MobKind,
    context: MobDamageContext,
    rng: &mut u64,
) -> Result<(Vec<ItemStack>, u8), MobError> {
    let mut drops = Vec::with_capacity(2);
    let mut add = |name: &str, count: u8| -> Result<(), MobError> {
        if count != 0 {
            let item = ItemId::from_name(name).ok_or(MobError::InvalidFacts)?;
            drops.push(
                ItemStack::try_new(item, count, ComponentPatch::empty())
                    .map_err(|_| MobError::InvalidFacts)?,
            );
        }
        Ok(())
    };
    match kind {
        MobKind::Sheep => {
            add(
                if context.burning {
                    "cooked_mutton"
                } else {
                    "mutton"
                },
                (1 + draw(rng, 2)) as u8,
            )?;
            add("white_wool", 1)?;
        }
        MobKind::Cow => {
            add("leather", draw(rng, 3) as u8)?;
            add(
                if context.burning {
                    "cooked_beef"
                } else {
                    "beef"
                },
                (1 + draw(rng, 3)) as u8,
            )?;
        }
        MobKind::Pig => add(
            if context.burning {
                "cooked_porkchop"
            } else {
                "porkchop"
            },
            (1 + draw(rng, 3)) as u8,
        )?,
        MobKind::Chicken => {
            add("feather", draw(rng, 3) as u8)?;
            add(
                if context.burning {
                    "cooked_chicken"
                } else {
                    "chicken"
                },
                1,
            )?;
        }
        MobKind::Zombie => {
            add("rotten_flesh", draw(rng, 3) as u8)?;
            if context.killed_by_player && draw(rng, 40) == 0 {
                let choice = draw(rng, 3);
                add(
                    match choice {
                        0 => "iron_ingot",
                        1 => "carrot",
                        _ if context.burning => "baked_potato",
                        _ => "potato",
                    },
                    1,
                )?;
            }
        }
    }
    let xp = if !context.killed_by_player {
        0
    } else if kind == MobKind::Zombie {
        5
    } else {
        (1 + draw(rng, 3)) as u8
    };
    Ok((drops, xp))
}

#[cfg(test)]
#[path = "mobs/tests.rs"]
mod tests;
