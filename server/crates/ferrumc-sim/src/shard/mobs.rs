//! Actual bounded mob spawning, AI, health and reserved death loot.

#![allow(clippy::result_large_err)]

use super::{is_valid_position, EntityTickContext, GameMode, SimShard};
use crate::mob_runtime::{MobPlayerProfile, MobRuntime, MAX_MOB_EVENTS, MAX_MOB_SPAWNS};
use crate::mobs::{
    MobActor, MobController, MobDamageContext, MobSpawnContext, MobTarget, MAX_ACTIVE_MOBS,
};
use crate::{
    DamageRequest, EntityData, EntitySpawn, EntityStore, EntityTransform, GameOutput,
    GroundItemDrop, MobEvent, MobRules, MobRuntimeError, MobSnapshot, MobSpawnAttempt,
    MobSpawnRefusal, ShardId,
};
use ferrumc_core::{EntityId, EntityUuid, PlayerId};
use ferrumc_math::Vec3;
use std::collections::{BTreeMap, VecDeque};
use std::num::NonZeroUsize;

impl SimShard {
    /// Selects one bounded legacy mob consumer, exclusive with native durability.
    /// Canonical base-only entity stores reserve their own global64 slots, never
    /// ground-item payload/receipt credits. Natural spawn light is supplied by a
    /// current verified App relighter; no missing light is fabricated here.
    pub fn enable_mobs(
        &mut self,
        peaceful: bool,
        shared_spawn: Vec3,
    ) -> Result<(), MobRuntimeError> {
        if self.player_ownership.is_some() || !is_valid_position(shared_spawn) {
            return Err(MobRuntimeError::OwnershipRequired);
        }
        if self.mobs.is_some() {
            return Err(MobRuntimeError::AlreadyInitialized);
        }
        let rules = MobRules::builtin()?;
        self.mobs = Some(MobRuntime {
            live: true,
            world: self.chunks.world(),
            dimension: self.chunks.dimension(),
            now: 0,
            peaceful,
            shared_spawn,
            controller: Some(MobController::new(rules.clone())),
            rules,
            stores: BTreeMap::new(),
            active: BTreeMap::new(),
            pending: VecDeque::new(),
            restores: VecDeque::new(),
            events: VecDeque::new(),
            profiles: BTreeMap::new(),
        });
        Ok(())
    }

    /// Updates a trusted immutable armor/wire projection for a genuine admitted
    /// player. App updates this owner synchronously before an inventory mutation;
    /// this mirror owns no armor stacks, health or player inventory authority.
    pub fn set_mob_player_profile(
        &mut self,
        player: PlayerId,
        wire_id: EntityId,
        armor: f32,
        toughness: f32,
    ) -> Result<(), MobRuntimeError> {
        if !self.players.contains_key(&player) || wire_id.get() < 2 {
            return Err(MobRuntimeError::InvalidInput);
        }
        DamageRequest::new(0.0, armor, toughness).map_err(|_| MobRuntimeError::InvalidInput)?;
        let runtime = self.mobs.as_ref().ok_or(MobRuntimeError::NotInitialized)?;
        runtime.check_live()?;
        if !runtime.profiles.contains_key(&player) && runtime.profiles.len() >= MAX_ACTIVE_MOBS {
            return Err(MobRuntimeError::Capacity("player profiles"));
        }
        if runtime
            .profiles
            .iter()
            .any(|(id, profile)| *id != player && profile.wire == wire_id)
            || runtime
                .pending
                .iter()
                .any(|spawn| spawn.identity.wire == wire_id)
            || runtime
                .restores
                .iter()
                .any(|(_, identity)| identity.wire == wire_id)
            || runtime.active.contains_key(&wire_id)
            || self.ground_wire_in_use(wire_id)
        {
            return Err(MobRuntimeError::InvalidIdentity);
        }
        self.mobs
            .as_mut()
            .ok_or(MobRuntimeError::NotInitialized)?
            .profiles
            .insert(
                player,
                MobPlayerProfile {
                    targetable: true,
                    wire: wire_id,
                    armor,
                    toughness,
                },
            );
        Ok(())
    }

    /// Withdraws the trusted AI target projection while App reserves an armor
    /// witness or lacks effect publication room. No missing profile is replaced
    /// with default armor; the actual AI target list excludes that player.
    /// Its genuine wire reservation remains until player departure.
    pub fn clear_mob_player_profile(&mut self, player: PlayerId) -> Result<(), MobRuntimeError> {
        let runtime = self.mobs.as_mut().ok_or(MobRuntimeError::NotInitialized)?;
        runtime.check_live()?;
        if let Some(profile) = runtime.profiles.get_mut(&player) {
            profile.targetable = false;
        }
        Ok(())
    }

    /// Queues one complete resident-light natural spawn proposal (pending bound16).
    /// Refusal retains the proposal. Accepted proposals may still fail at the
    /// actual tick, returning their identity through `SpawnRefused` publication.
    pub fn enqueue_mob_spawn(
        &mut self,
        mut attempt: MobSpawnAttempt,
    ) -> Result<(), MobSpawnRefusal> {
        let checked = (|| {
            let runtime = self.mobs.as_ref().ok_or(MobRuntimeError::NotInitialized)?;
            runtime.check_live()?;
            if runtime.pending.len() >= MAX_MOB_SPAWNS
                || runtime.active.len() + runtime.pending.len() + runtime.restores.len()
                    >= MAX_ACTIVE_MOBS
            {
                return Err(MobRuntimeError::Capacity("population or pending spawns"));
            }
            if !is_valid_position(attempt.position) || attempt.identity.wire.get() < 2 {
                return Err(MobRuntimeError::InvalidInput);
            }
            if runtime.identity_used(attempt.identity.uuid, attempt.identity.wire)
                || self.ground_identity_in_use(attempt.identity.uuid, attempt.identity.wire)
            {
                return Err(MobRuntimeError::InvalidIdentity);
            }
            Ok(())
        })();
        if let Err(reason) = checked {
            return Err(MobSpawnRefusal { reason, attempt });
        }
        let Some(runtime) = self.mobs.as_mut() else {
            unreachable!("checked mob runtime");
        };
        attempt.admitted_revision = self.mob_light_revision;
        runtime.pending.push_back(attempt);
        Ok(())
    }

    /// Captures at most64 complete canonical base/controller native records.
    /// Pending natural/restore proposals refuse the whole capture. No dirty fact
    /// or runtime ownership clears; App saves this set with its item set before
    /// deleting absent records, under the existing generation binding/barrier.
    pub fn capture_mob_records(
        &self,
    ) -> Result<Vec<crate::mob_persistence::MobRecord>, MobRuntimeError> {
        let runtime = self.mobs.as_ref().ok_or(MobRuntimeError::NotInitialized)?;
        runtime.check_live()?;
        if !runtime.pending.is_empty() || !runtime.restores.is_empty() {
            return Err(MobRuntimeError::Capacity("unsettled mob proposals"));
        }
        let controller = runtime
            .controller
            .as_ref()
            .ok_or(MobRuntimeError::OwnershipRequired)?;
        runtime
            .active
            .keys()
            .map(|wire| {
                let snapshot = runtime
                    .snapshot(*wire)
                    .ok_or(MobRuntimeError::InvalidIdentity)?;
                let state = controller.capture_state(*wire, runtime.now)?;
                crate::mob_persistence::encode_mob_entity(snapshot.base, state, &runtime.rules)
            })
            .collect()
    }

    /// Enumerates the bounded canonical columns of all live/pending/saved mobs.
    /// App keeps their actual entity tickets; this method performs no I/O/loading.
    pub fn mob_chunks(
        &self,
    ) -> Result<std::collections::BTreeSet<ferrumc_math::ChunkPos>, MobRuntimeError> {
        let runtime = self.mobs.as_ref().ok_or(MobRuntimeError::NotInitialized)?;
        runtime.check_live()?;
        let mut chunks = std::collections::BTreeSet::new();
        for wire in runtime.active.keys() {
            chunks.insert(
                runtime
                    .snapshot(*wire)
                    .ok_or(MobRuntimeError::InvalidIdentity)?
                    .base
                    .chunk(),
            );
        }
        for attempt in &runtime.pending {
            chunks.insert(EntityTransform::try_new(attempt.position, 0.0, 0.0)?.chunk());
        }
        chunks.extend(
            runtime
                .restores
                .iter()
                .map(|(proposal, _)| proposal.transform().chunk()),
        );
        Ok(chunks)
    }

    /// Retains one exact decoded saved mob until a real resident tick admission.
    /// Fresh identity must preserve the saved UUID; wire ID is process allocated.
    /// Queue16/global64/event256 bounds and all global identities remain explicit.
    pub fn enqueue_mob_restore(
        &mut self,
        proposal: crate::MobRestoreProposal,
        identity: crate::MobIdentity,
    ) -> Result<(), crate::MobRestoreRefusal> {
        let checked = (|| {
            let runtime = self.mobs.as_ref().ok_or(MobRuntimeError::NotInitialized)?;
            runtime.check_live()?;
            if proposal.key().world() != runtime.world
                || proposal.key().dimension() != runtime.dimension
                || proposal.key().uuid() != identity.uuid
                || identity.wire.get() < 2
            {
                return Err(MobRuntimeError::InvalidInput);
            }
            proposal.state.validate(&runtime.rules)?;
            if runtime.restores.len() >= MAX_MOB_SPAWNS
                || runtime.active.len() + runtime.pending.len() + runtime.restores.len()
                    >= MAX_ACTIVE_MOBS
            {
                return Err(MobRuntimeError::Capacity("population or pending restores"));
            }
            if runtime.identity_used(identity.uuid, identity.wire)
                || self.ground_identity_in_use(identity.uuid, identity.wire)
            {
                return Err(MobRuntimeError::InvalidIdentity);
            }
            Ok(())
        })();
        if let Err(reason) = checked {
            return Err(crate::MobRestoreRefusal {
                reason,
                proposal,
                identity,
            });
        }
        let Some(runtime) = self.mobs.as_mut() else {
            unreachable!("checked mob runtime");
        };
        runtime.restores.push_back((proposal, identity));
        Ok(())
    }

    /// Pages current actual mob registrations in ascending genuine wire ID.
    /// Snapshot copies have no health/controller authority and refuse this query.
    pub fn mob_snapshots(
        &self,
        after: Option<EntityId>,
        max: usize,
    ) -> Result<Vec<MobSnapshot>, MobRuntimeError> {
        if max == 0 || max > MAX_ACTIVE_MOBS {
            return Err(MobRuntimeError::InvalidInput);
        }
        let runtime = self.mobs.as_ref().ok_or(MobRuntimeError::NotInitialized)?;
        runtime.check_live()?;
        runtime
            .active
            .keys()
            .filter(|wire| after.is_none_or(|last| **wire > last))
            .take(max)
            .map(|wire| {
                runtime
                    .snapshot(*wire)
                    .ok_or(MobRuntimeError::InvalidIdentity)
            })
            .collect()
    }

    /// Moves at most256 fixed-size events out of this actual sole consumer.
    pub fn take_mob_events(&mut self, max: usize) -> Result<Vec<MobEvent>, MobRuntimeError> {
        if max == 0 || max > MAX_MOB_EVENTS {
            return Err(MobRuntimeError::InvalidInput);
        }
        let runtime = self.mobs.as_mut().ok_or(MobRuntimeError::NotInitialized)?;
        runtime.check_live()?;
        Ok(runtime
            .events
            .drain(..max.min(runtime.events.len()))
            .collect())
    }

    pub(super) fn ground_wire_in_use(&self, wire: EntityId) -> bool {
        self.entities
            .as_ref()
            .is_some_and(|store| store.handle_for_wire_id(wire).is_some())
            || self.ground_items.as_ref().is_some_and(|ground| {
                ground.identities.iter().any(|id| id.wire_id == wire)
                    || ground
                        .regions
                        .values()
                        .any(|store| store.handle_for_wire_id(wire).is_some())
                    || ground
                        .restores
                        .iter()
                        .any(|(_, restore)| restore.wire_id == wire)
                    || ground
                        .offers
                        .values()
                        .any(|offer| offer.base.wire_id() == wire)
            })
    }
    pub(super) fn ground_identity_in_use(&self, uuid: EntityUuid, wire: EntityId) -> bool {
        self.players
            .keys()
            .any(|player| player.as_uuid() == uuid.as_uuid())
            || self.ground_wire_in_use(wire)
            || self
                .entities
                .as_ref()
                .is_some_and(|store| store.handle_for_uuid(uuid).is_some())
            || self.ground_items.as_ref().is_some_and(|ground| {
                ground.identities.iter().any(|id| id.uuid == uuid)
                    || ground
                        .regions
                        .values()
                        .any(|store| store.handle_for_uuid(uuid).is_some())
                    || ground
                        .restores
                        .iter()
                        .any(|(_, restore)| restore.proposal.key().uuid() == uuid)
                    || ground
                        .offers
                        .values()
                        .any(|offer| offer.base.uuid() == uuid)
            })
    }

    pub(super) fn begin_mob_tick(&mut self) {
        if let Some(runtime) = self.mobs.as_mut() {
            assert!(
                runtime.check_live().is_ok(),
                "mob snapshot has no tick authority"
            );
            assert!(
                runtime.world == self.chunks.world()
                    && runtime.dimension == self.chunks.dimension(),
                "mob scope changed before inbox drain"
            );
            runtime.now = runtime
                .now
                .checked_add(1)
                .unwrap_or_else(|| panic!("logical mob tick overflow"));
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "One bounded canonical AI phase retains explicit publication and health order"
    )]
    pub(super) fn drive_mobs(&mut self, outputs: &mut Vec<GameOutput>) {
        let Some(mut runtime) = self.mobs.take() else {
            return;
        };
        runtime
            .profiles
            .retain(|player, _| self.players.contains_key(player));
        runtime.stores.retain(|_, store| !store.is_empty());
        for _ in 0..runtime.restores.len().min(MAX_MOB_SPAWNS) {
            if runtime.events.len() >= MAX_MOB_EVENTS {
                break;
            }
            let Some((proposal, identity)) = runtime.restores.pop_front() else {
                break;
            };
            match self.restore_mob(&mut runtime, proposal, identity) {
                Ok(snapshot) => runtime.events.push_back(MobEvent::Spawn(snapshot)),
                Err(_) => runtime.restores.push_back((proposal, identity)),
            }
        }
        for _ in 0..4 {
            if runtime.events.len() >= MAX_MOB_EVENTS {
                break;
            }
            let Some(attempt) = runtime.pending.pop_front() else {
                break;
            };
            match self.spawn_mob(&mut runtime, attempt) {
                Ok(snapshot) => runtime.events.push_back(MobEvent::Spawn(snapshot)),
                Err(reason) => runtime.events.push_back(MobEvent::SpawnRefused {
                    identity: attempt.identity,
                    reason,
                }),
            }
        }
        let actors: Vec<_> = runtime
            .active
            .keys()
            .filter_map(|wire| runtime.snapshot(*wire))
            .filter(|snapshot| self.chunks.is_loaded(snapshot.base.chunk()))
            .filter_map(|snapshot| {
                MobActor::new(
                    snapshot.base.wire_id(),
                    snapshot.base.transform().position(),
                    snapshot.base.transform().yaw(),
                )
                .ok()
            })
            .collect();
        let targets: Vec<_> = runtime
            .profiles
            .iter()
            .filter_map(|(player, profile)| {
                if !profile.targetable {
                    return None;
                }
                let actor = self.players.get(player)?;
                if !actor.is_alive()
                    || matches!(actor.game_mode, GameMode::Creative | GameMode::Spectator)
                {
                    return None;
                }
                MobTarget::new(profile.wire, actor.position, true).ok()
            })
            .collect();
        let step = runtime
            .controller
            .as_mut()
            .unwrap_or_else(|| panic!("live mob controller missing"))
            .step(runtime.now, &actors, &targets, |pos| {
                self.chunks
                    .get(pos.to_chunk_pos())
                    .and_then(|chunk| chunk.get_block(pos))
            })
            .unwrap_or_else(|error| panic!("bounded genuine mob step refused: {error}"));
        for movement in step.movements() {
            if runtime.events.len() >= MAX_MOB_EVENTS {
                break;
            }
            let Some(before) = runtime.snapshot(movement.id()) else {
                panic!("controller moved unknown canonical mob");
            };
            assert_eq!(
                before.base.transform().position(),
                movement.before(),
                "controller beforeimage differs from canonical transform"
            );
            let Ok(transform) = EntityTransform::try_new(
                movement.after(),
                movement.yaw(),
                before.base.transform().pitch(),
            ) else {
                continue;
            };
            if let Ok(base) = move_mob(&mut runtime, &self.chunks, movement.id(), transform) {
                runtime.events.push_back(MobEvent::Move(base));
            }
        }
        for attack in step.attacks() {
            let Some((&player, profile)) = runtime
                .profiles
                .iter()
                .find(|(_, profile)| profile.targetable && profile.wire == attack.target())
            else {
                continue;
            };
            let Some(snapshot) = runtime.snapshot(attack.attacker()) else {
                continue;
            };
            let Some(actor) = self.players.get(&player) else {
                continue;
            };
            if !actor.is_alive()
                || matches!(actor.game_mode, GameMode::Creative | GameMode::Spectator)
                || (actor.position - snapshot.base.transform().position()).length_squared() > 9.0
            {
                continue;
            }
            let Some(vitals) = actor.survival.as_ref().map(|state| state.vitals.snapshot()) else {
                continue;
            };
            let Ok(request) =
                DamageRequest::new(attack.base_damage(), profile.armor, profile.toughness)
                    .and_then(|request| request.with_exhaustion(0.1))
            else {
                continue;
            };
            let raw = request.incremental_amount(vitals);
            let damage = self.damage_player(player, request, outputs);
            if damage > 0.0 {
                outputs.push(GameOutput::MobPlayerHurt {
                    player,
                    attacker: attack.attacker(),
                    damage,
                    raw_damage: raw,
                });
            }
        }
        self.mobs = Some(runtime);
    }

    fn restore_mob(
        &mut self,
        runtime: &mut MobRuntime,
        proposal: crate::MobRestoreProposal,
        identity: crate::MobIdentity,
    ) -> Result<MobSnapshot, MobRuntimeError> {
        if !self.chunks.is_loaded(proposal.transform.chunk()) {
            return Err(MobRuntimeError::InvalidInput);
        }
        if runtime.active.len() >= MAX_ACTIVE_MOBS
            || runtime.identity_used(identity.uuid, identity.wire)
            || self.ground_identity_in_use(identity.uuid, identity.wire)
        {
            return Err(MobRuntimeError::InvalidIdentity);
        }
        let owner = crate::ShardPartitioner::for_chunk(
            runtime.world,
            runtime.dimension,
            proposal.transform.chunk(),
        );
        ensure_mob_region(runtime, owner)?;
        let entity = EntitySpawn::new(
            runtime.world,
            runtime.dimension,
            identity.uuid,
            identity.wire,
            runtime.rules.attributes(proposal.state.kind).entity_type(),
            proposal.transform,
        );
        let store = runtime
            .stores
            .get_mut(&owner)
            .ok_or(MobRuntimeError::InvalidInput)?;
        let handle = EntityTickContext::new(store, &self.chunks).spawn(entity)?;
        let restored = runtime
            .controller
            .as_mut()
            .ok_or(MobRuntimeError::OwnershipRequired)?
            .restore_state(identity.wire, runtime.now.saturating_sub(1), proposal.state);
        if let Err(reason) = restored {
            EntityTickContext::new(store, &self.chunks)
                .remove(&handle)
                .unwrap_or_else(|error| panic!("refused saved mob rollback failed: {error}"));
            return Err(reason.into());
        }
        runtime.active.insert(identity.wire, handle);
        runtime
            .snapshot(identity.wire)
            .ok_or(MobRuntimeError::InvalidIdentity)
    }

    fn spawn_mob(
        &mut self,
        runtime: &mut MobRuntime,
        attempt: MobSpawnAttempt,
    ) -> Result<MobSnapshot, MobRuntimeError> {
        if attempt.admitted_revision != self.mob_light_revision {
            return Err(MobRuntimeError::InvalidInput);
        }
        if runtime.active.len() >= MAX_ACTIVE_MOBS {
            return Err(MobRuntimeError::Capacity("population"));
        }
        if self.ground_identity_in_use(attempt.identity.uuid, attempt.identity.wire)
            || runtime.identity_used(attempt.identity.uuid, attempt.identity.wire)
        {
            return Err(MobRuntimeError::InvalidIdentity);
        }
        if runtime.dimension.get() != 0
            || !is_valid_position(attempt.position)
            || (attempt.position - runtime.shared_spawn).length_squared() <= 576.0
        {
            return Err(MobRuntimeError::InvalidInput);
        }
        let nearest = self
            .players
            .values()
            .filter(|actor| {
                actor.is_alive()
                    && !matches!(actor.game_mode, GameMode::Creative | GameMode::Spectator)
            })
            .map(|actor| (actor.position - attempt.position).length_squared())
            .reduce(f64::min)
            .ok_or(MobRuntimeError::InvalidInput)?;
        let context = MobSpawnContext::new(
            attempt.light,
            nearest,
            runtime.peaceful,
            attempt.sky_roll,
            attempt.dark_roll,
        )?;
        runtime
            .rules
            .validate_spawn(attempt.kind, attempt.position, context, |pos| {
                self.chunks
                    .get(pos.to_chunk_pos())
                    .and_then(|chunk| chunk.get_block(pos))
            })?;
        let attributes = runtime.rules.attributes(attempt.kind);
        for &wire in runtime.active.keys() {
            let existing = runtime
                .snapshot(wire)
                .ok_or(MobRuntimeError::InvalidIdentity)?;
            let other = runtime.rules.attributes(existing.kind);
            if bodies_overlap(
                attempt.position,
                attributes.width(),
                attributes.height(),
                existing.base.transform().position(),
                other.width(),
                other.height(),
            ) {
                return Err(MobRuntimeError::Rules(crate::mobs::MobError::SpawnRefused));
            }
        }
        let transform = EntityTransform::try_new(attempt.position, 0.0, 0.0)?;
        let owner =
            crate::ShardPartitioner::for_chunk(runtime.world, runtime.dimension, transform.chunk());
        ensure_mob_region(runtime, owner)?;
        let proposal = EntitySpawn::new(
            runtime.world,
            runtime.dimension,
            attempt.identity.uuid,
            attempt.identity.wire,
            attributes.entity_type(),
            transform,
        );
        let store = runtime
            .stores
            .get_mut(&owner)
            .ok_or(MobRuntimeError::InvalidInput)?;
        let handle = EntityTickContext::new(store, &self.chunks).spawn(proposal)?;
        let fact = runtime
            .controller
            .as_mut()
            .ok_or(MobRuntimeError::OwnershipRequired)?
            .admit(attempt.identity.wire, attempt.kind, attempt.seed);
        if let Err(reason) = fact {
            EntityTickContext::new(store, &self.chunks)
                .remove(&handle)
                .unwrap_or_else(|error| panic!("refused mob admission rollback failed: {error}"));
            return Err(reason.into());
        }
        runtime.active.insert(attempt.identity.wire, handle);
        runtime
            .snapshot(attempt.identity.wire)
            .ok_or(MobRuntimeError::InvalidIdentity)
    }

    pub(super) fn attack_mob(
        &mut self,
        player: PlayerId,
        target: EntityId,
        base_damage: f32,
        attack_speed: f32,
        token: u64,
        outputs: &mut Vec<GameOutput>,
    ) {
        let result = self.apply_mob_attack(player, target, base_damage, attack_speed, token);
        let (damage, experience) = result.unwrap_or((0.0, 0));
        outputs.push(GameOutput::MobAttackResolved {
            player,
            target,
            token,
            accepted: damage > 0.0,
            damage,
            experience,
        });
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Whole lethal-health and ground-loot admission remains explicit before commit"
    )]
    fn apply_mob_attack(
        &mut self,
        player: PlayerId,
        target: EntityId,
        base_damage: f32,
        attack_speed: f32,
        token: u64,
    ) -> Result<(f32, u8), MobRuntimeError> {
        let actor = self
            .players
            .get_mut(&player)
            .ok_or(MobRuntimeError::InvalidInput)?;
        let state = actor
            .survival
            .as_mut()
            .ok_or(MobRuntimeError::OwnershipRequired)?;
        if token == 0 || token <= state.last_attack_token {
            return Err(MobRuntimeError::InvalidInput);
        }
        state.last_attack_token = token;
        if state.vitals.is_dead()
            || actor.game_mode == GameMode::Spectator
            || !attack_speed.is_finite()
            || !(0.1..=1024.0).contains(&attack_speed)
            || !base_damage.is_finite()
            || base_damage <= 0.0
        {
            return Err(MobRuntimeError::InvalidInput);
        }
        let position = actor.position;
        let exhausts = actor.game_mode != GameMode::Creative;
        let strength = ((f32::from(state.attack_strength_ticks) + 0.5) / (20.0 / attack_speed))
            .clamp(0.0, 1.0);
        let raw = base_damage * (0.2 + 0.8 * strength * strength);
        let runtime = self.mobs.as_mut().ok_or(MobRuntimeError::NotInitialized)?;
        runtime.check_live()?;
        let before = runtime
            .snapshot(target)
            .ok_or(MobRuntimeError::InvalidIdentity)?;
        if !self.chunks.is_loaded(before.base.chunk())
            || (position - before.base.transform().position()).length_squared() > 9.0
        {
            return Err(MobRuntimeError::InvalidInput);
        }
        let controller = runtime
            .controller
            .as_mut()
            .ok_or(MobRuntimeError::OwnershipRequired)?;
        let plan = controller.prepare_damage(
            runtime.now,
            target,
            raw,
            MobDamageContext::new(false, true),
        )?;
        let is_death = plan.is_death();
        if runtime.events.len() + if is_death { 2 } else { 1 } > MAX_MOB_EVENTS {
            return Err(MobRuntimeError::Capacity("damage publication"));
        }
        let mut charges = Vec::with_capacity(plan.drops().len());
        if !plan.drops().is_empty() {
            let ground = self
                .ground_items
                .as_ref()
                .ok_or(MobRuntimeError::LootCapacity)?;
            ground
                .reserve_drop_bytes(plan.drops())
                .map_err(|_| MobRuntimeError::LootCapacity)?;
            ground
                .check_region(ground.owner_for_position(before.base.transform().position()))
                .map_err(|_| MobRuntimeError::LootCapacity)?;
            for stack in plan.drops() {
                charges.push(
                    ferrumc_items::persisted_stack_size(stack)
                        .map_err(|_| MobRuntimeError::LootCapacity)?
                        + crate::NATIVE_ITEM_ENTITY_HEADER_BYTES,
                );
            }
        }
        let receipt = controller
            .apply_damage(plan)
            .map_err(|refusal| MobRuntimeError::Rules(refusal.error()))?;
        let damage = before.health - receipt.fact().health();
        let experience = receipt.experience();
        runtime.events.push_back(MobEvent::Hurt(MobSnapshot {
            base: before.base,
            kind: before.kind,
            health: receipt.fact().health(),
        }));
        let drops = receipt.into_drops();
        if !drops.is_empty() {
            let ground = self
                .ground_items
                .as_mut()
                .unwrap_or_else(|| panic!("reserved mob death ground owner missing"));
            for (stack, size) in drops.into_iter().zip(charges) {
                ground.admit_drop(
                    size,
                    GroundItemDrop::new(before.base.transform().position(), stack, 10, None, None),
                );
            }
        }
        if is_death {
            let handle = runtime
                .active
                .remove(&target)
                .unwrap_or_else(|| panic!("dead canonical mob handle missing"));
            let store = runtime
                .stores
                .get_mut(&handle.owner())
                .unwrap_or_else(|| panic!("dead canonical mob store missing"));
            let base = EntityTickContext::new(store, &self.chunks)
                .remove(&handle)
                .unwrap_or_else(|error| panic!("reserved mob removal refused: {error}"));
            runtime
                .controller
                .as_mut()
                .unwrap_or_else(|| panic!("live mob controller missing"))
                .remove(target);
            runtime.events.push_back(MobEvent::Remove(base));
        }
        let state = self
            .players
            .get_mut(&player)
            .and_then(|actor| actor.survival.as_mut())
            .unwrap_or_else(|| panic!("committed mob attacker missing"));
        state.attack_strength_ticks = 0;
        if exhausts {
            state
                .vitals
                .add_exhaustion(0.1)
                .unwrap_or_else(|error| panic!("finite mob attack exhaustion refused: {error}"));
        }
        Ok((damage, experience))
    }
}

fn ensure_mob_region(runtime: &mut MobRuntime, owner: ShardId) -> Result<(), MobRuntimeError> {
    if !runtime.stores.contains_key(&owner) {
        if runtime.stores.len() >= MAX_ACTIVE_MOBS {
            return Err(MobRuntimeError::Capacity("regional stores"));
        }
        let slots = NonZeroUsize::new(MAX_ACTIVE_MOBS).ok_or(MobRuntimeError::InvalidInput)?;
        runtime
            .stores
            .insert(owner, EntityStore::new(owner, slots)?);
    }
    Ok(())
}
fn move_mob(
    runtime: &mut MobRuntime,
    chunks: &crate::LoadedChunkMap,
    wire: EntityId,
    transform: EntityTransform,
) -> Result<EntityData, MobRuntimeError> {
    let handle = runtime
        .active
        .get(&wire)
        .ok_or(MobRuntimeError::InvalidIdentity)?
        .clone();
    let owner =
        crate::ShardPartitioner::for_chunk(runtime.world, runtime.dimension, transform.chunk());
    if owner == handle.owner() {
        let store = runtime
            .stores
            .get_mut(&owner)
            .ok_or(MobRuntimeError::InvalidIdentity)?;
        EntityTickContext::new(store, chunks).update_transform(&handle, transform)?;
    } else {
        ensure_mob_region(runtime, owner)?;
        let mut destination = runtime
            .stores
            .remove(&owner)
            .ok_or(MobRuntimeError::InvalidIdentity)?;
        let result = (|| {
            let source = runtime
                .stores
                .get_mut(&handle.owner())
                .ok_or(MobRuntimeError::InvalidIdentity)?;
            EntityTickContext::new(source, chunks)
                .migrate_base_to(
                    &mut EntityTickContext::new(&mut destination, chunks),
                    &handle,
                    transform,
                )
                .map_err(MobRuntimeError::from)
        })();
        runtime.stores.insert(owner, destination);
        runtime.active.insert(wire, result?);
    }
    let handle = runtime
        .active
        .get(&wire)
        .ok_or(MobRuntimeError::InvalidIdentity)?;
    Ok(*runtime
        .stores
        .get(&handle.owner())
        .ok_or(MobRuntimeError::InvalidIdentity)?
        .get(handle)?)
}
fn bodies_overlap(a: Vec3, aw: f64, ah: f64, b: Vec3, bw: f64, bh: f64) -> bool {
    (a.x - b.x).abs() < aw.midpoint(bw)
        && (a.z - b.z).abs() < aw.midpoint(bw)
        && a.y < b.y + bh
        && b.y < a.y + ah
}
