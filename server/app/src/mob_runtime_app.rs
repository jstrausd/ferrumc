//! Bounded legacy mob wiring using current resident light and canonical owners.
use ferrumc_core::{EntityId, EntityUuid, PlayerId};
use ferrumc_math::{BlockPos, Vec3};
use ferrumc_proto::generated::play::ClientboundPlayPacket;
use ferrumc_session::SessionRouter;
use ferrumc_sim::{MobEvent, MobIdentity, MobKind, MobSpawnAttempt, MobSpawnLight, SimShard};
use std::collections::{BTreeMap, VecDeque};
use tokio::sync::oneshot;

struct SpawnRequest {
    player: PlayerId,
    kind: MobKind,
    position: Vec3,
    expires: u64,
}

pub(crate) struct MobRuntime {
    pub(crate) view: crate::mob_view::MobView,
    requests: VecDeque<SpawnRequest>,
    observers: BTreeMap<EntityUuid, PlayerId>,
    attacks: BTreeMap<PlayerId, (u64, EntityId, oneshot::Sender<bool>)>,
    random: u64,
    column: usize,
}

impl MobRuntime {
    pub(crate) fn new(seed: i64) -> anyhow::Result<Self> {
        Ok(Self {
            view: crate::mob_view::MobView::new()?,
            requests: VecDeque::new(),
            observers: BTreeMap::new(),
            attacks: BTreeMap::new(),
            random: seed as u64,
            column: 0,
        })
    }

    pub(crate) fn request_spawn(
        &mut self,
        player: PlayerId,
        kind: MobKind,
        position: Vec3,
        now: u64,
    ) -> bool {
        if self.requests.len() + self.observers.len() >= 16 {
            return false;
        }
        self.requests.push_back(SpawnRequest {
            player,
            kind,
            position,
            expires: now.saturating_add(100),
        });
        true
    }

    pub(crate) fn track_attack(
        &mut self,
        player: PlayerId,
        target: EntityId,
        token: u64,
        reply: oneshot::Sender<bool>,
    ) -> Result<(), oneshot::Sender<bool>> {
        if token == 0 || self.attacks.len() >= 4096 || self.attacks.contains_key(&player) {
            return Err(reply);
        }
        self.attacks.insert(player, (token, target, reply));
        Ok(())
    }

    pub(crate) fn fail_attack(&mut self, player: PlayerId) {
        if let Some((_, _, reply)) = self.attacks.remove(&player) {
            let _ = reply.send(false);
        }
    }

    pub(crate) fn resolve(&mut self, output: &ferrumc_sim::GameOutput) {
        match output {
            ferrumc_sim::GameOutput::MobAttackResolved {
                player,
                target,
                token,
                accepted,
                ..
            } if self
                .attacks
                .get(player)
                .is_some_and(|(t, id, _)| t == token && id == target) =>
            {
                if let Some((_, _, reply)) = self.attacks.remove(player) {
                    let _ = reply.send(*accepted);
                }
            }
            ferrumc_sim::GameOutput::PlayerDespawned { player } => self.forget(*player),
            _ => {}
        }
    }

    pub(crate) fn forget(&mut self, player: PlayerId) {
        self.fail_attack(player);
        self.requests.retain(|request| request.player != player);
        self.observers.retain(|_, requester| *requester != player);
        self.view.forget(player);
    }

    pub(crate) fn events(
        &mut self,
        shard: &mut SimShard,
        router: &mut SessionRouter,
    ) -> anyhow::Result<()> {
        for event in shard.take_mob_events(256)? {
            match event {
                MobEvent::Spawn(snapshot) => {
                    self.view.snapshot(&snapshot)?;
                    if let Some(player) = self.observers.remove(&snapshot.base().uuid()) {
                        feedback(router, player, "Mob spawned");
                    }
                }
                MobEvent::Move(base) => self.view.moved(router, base),
                MobEvent::Hurt(snapshot) => self.view.changed(router, &snapshot)?,
                MobEvent::Remove(base) => self.view.remove(router, base),
                MobEvent::SpawnRefused { identity, reason } => {
                    if let Some(player) = self.observers.remove(&identity.uuid()) {
                        feedback(router, player, &format!("Mob spawn refused: {reason}"));
                    }
                    tracing::debug!(%reason, wire=identity.wire_id().get(), "resident mob spawn refused");
                }
            }
        }
        Ok(())
    }

    /// At most one explicit request and one natural attempt per ten owner ticks.
    /// No candidate initiates a load or substitutes guessed light for absent data.
    pub(crate) fn spawn_tick(
        &mut self,
        now: u64,
        day_time: i64,
        raining: bool,
        light: &crate::lighting::Relighter,
        shard: &mut SimShard,
        router: &mut SessionRouter,
    ) -> anyhow::Result<()> {
        if let Some(request) = self.requests.pop_front() {
            let at = BlockPos::new(
                request.position.x.floor() as i32,
                request.position.y.floor() as i32,
                request.position.z.floor() as i32,
            );
            if let Some(illumination) = light.resident_light(at) {
                self.attempt(
                    request.kind,
                    request.position,
                    illumination,
                    day_time,
                    raining,
                    Some(request.player),
                    shard,
                    router,
                )?;
            } else if now >= request.expires || !router.is_player_connected(request.player) {
                feedback(
                    router,
                    request.player,
                    "Mob spawn refused: resident light unavailable",
                );
            } else {
                self.requests.push_back(request);
            }
        }
        if shard.player_count() == 0
            || !now.is_multiple_of(10)
            || shard.mob_snapshots(None, 64)?.len() >= 48
        {
            return Ok(());
        }
        let columns = light.resident_columns();
        if columns.is_empty() {
            return Ok(());
        }
        self.column = self.column.wrapping_add(1);
        let column = columns[self.column % columns.len()];
        let random = self.next_random();
        let x = column.x() * 16 + (random & 15) as i32;
        let z = column.z() * 16 + ((random >> 4) & 15) as i32;
        let Some(chunk) = shard.loaded_chunks().get(column) else {
            return Ok(());
        };
        // One candidate column, at most384 resident block reads. The owner does
        // the actual pinned terrain/support/obstruction checks before admission.
        let mut surface = None;
        for y in (-64..=318).rev() {
            let Some(state) = chunk.get_block(BlockPos::new(x, y, z)) else {
                continue;
            };
            if !state.is_air() {
                surface = Some(y + 1);
                break;
            }
        }
        let Some(y) = surface else {
            return Ok(());
        };
        let at = BlockPos::new(x, y, z);
        let Some(illumination) = light.resident_light(at) else {
            return Ok(());
        };
        let raw = illumination
            .raw_brightness(darkening(day_time, raining)?)
            .ok_or_else(|| anyhow::anyhow!("invalid resident mob light"))?;
        let kind = if raw <= 8 {
            MobKind::Zombie
        } else {
            [MobKind::Sheep, MobKind::Cow, MobKind::Pig, MobKind::Chicken]
                [((random >> 8) & 3) as usize]
        };
        self.attempt(
            kind,
            Vec3::new(f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5),
            illumination,
            day_time,
            raining,
            None,
            shard,
            router,
        )
    }

    #[allow(clippy::too_many_arguments)] // one immutable light/position proposal and its owner/identity authorities
    fn attempt(
        &mut self,
        kind: MobKind,
        position: Vec3,
        illumination: crate::light_view::ResidentLight,
        day_time: i64,
        raining: bool,
        requester: Option<PlayerId>,
        shard: &mut SimShard,
        router: &mut SessionRouter,
    ) -> anyhow::Result<()> {
        let darken = darkening(day_time, raining)?;
        let raw = illumination
            .raw_brightness(darken)
            .ok_or_else(|| anyhow::anyhow!("invalid resident mob light"))?;
        let light = MobSpawnLight::new(illumination.sky(), illumination.block(), raw)?;
        let random = self.next_random();
        let identity = MobIdentity::new(
            EntityUuid::from(uuid::Uuid::new_v4()),
            EntityId::new(router.allocate_non_player_entity_id()?),
        );
        let proposal = MobSpawnAttempt::new(
            identity,
            kind,
            position,
            light,
            ((random >> 16) & 31) as u8,
            ((random >> 21) & 7) as u8,
            random,
        );
        match shard.enqueue_mob_spawn(proposal) {
            Ok(()) => {
                if let Some(player) = requester {
                    self.observers.insert(identity.uuid(), player);
                }
            }
            Err(refusal) => {
                if let Some(player) = requester {
                    feedback(
                        router,
                        player,
                        &format!("Mob spawn refused: {}", refusal.reason()),
                    );
                }
            }
        }
        Ok(())
    }

    fn next_random(&mut self) -> u64 {
        self.random = self.random.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.random;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

fn feedback(router: &SessionRouter, player: PlayerId, message: &str) {
    router.send_play_packet_to(
        player,
        ClientboundPlayPacket::SystemChat(ferrumc_proto::generated::play::SystemChat::new(
            {
                let mut text = ferrumc_nbt::NbtCompound::new();
                text.push("text", ferrumc_nbt::NbtTag::String(message.to_owned()));
                ferrumc_nbt::NbtTag::Compound(text)
            },
            false,
        )),
    );
}

fn darkening(day_time: i64, raining: bool) -> anyhow::Result<u8> {
    if !raining {
        return Ok(crate::light_view::sky_darken(day_time));
    }
    crate::light_view::sky_darken_weather(day_time, 1.0, 0.0)
        .ok_or_else(|| anyhow::anyhow!("invalid mob weather"))
}
