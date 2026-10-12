//! NET-04.2e2 test driver: a driver-like loop over one genuine native shard
//! (memory or redb ownership in counter mode) and the router. It only uses
//! public simulation and session APIs: lane items, genuine work execution,
//! intents, and the session's closures. No fact, status or completion is
//! invented; storage refusals come from real store limits or real concurrent
//! writes.

use ferrumc_core::{EntityId, EntityUuid, PlayerId};
use ferrumc_math::{ChunkPos, Vec3};
use ferrumc_session::{
    NativeDetachOutcome, NativeRetentionStep, NativeRetentionToken, NativeShardReceiver,
    SessionRouter,
};
use ferrumc_sim::{
    EntityHandle, ItemEntityCounters, OwnedPlayerHandle, OwnedPlayerInput, OwnershipIntent,
    PlayerOwnershipTickOutcome,
};
use ferrumc_storage::{
    EntityOwnershipCasRequest, EntityOwnershipProposal, EntityRecord, OwnershipTransferId,
    PersistentEntityKey, PersistentEntityRecord, SchemaVersion,
};

use super::{generation, scope, SimEnv};

/// Protocol id of `minecraft:item` in the pinned 1.21.8 registry.
const ITEM_ENTITY_TYPE: i32 = 69;

pub fn item_uuid(n: u8) -> EntityUuid {
    EntityUuid::from_bytes([n; 16])
}

/// Independent fixed FCIE envelope of one plain stone stack (FCIS schema 1,
/// no components); the simulation's own restore validates every field.
fn item_record(n: u8, count: u8, position: Vec3) -> PersistentEntityRecord {
    let mut stack = b"FCIS".to_vec();
    stack.extend_from_slice(&1_u16.to_be_bytes());
    stack.push(1);
    let name = b"minecraft:stone";
    stack.extend_from_slice(&u16::try_from(name.len()).unwrap().to_be_bytes());
    stack.extend_from_slice(name);
    stack.extend_from_slice(&[count, 0, 0]);
    let counters = ItemEntityCounters::new(-32768, 0, 5);
    let mut bytes = b"FCIE\0\x01".to_vec();
    bytes.extend_from_slice(&ITEM_ENTITY_TYPE.to_be_bytes());
    bytes.extend_from_slice(&scope().world().get().to_be_bytes());
    bytes.extend_from_slice(&scope().dimension().get().to_be_bytes());
    bytes.extend_from_slice(&item_uuid(n).to_bytes());
    for axis in [position.x, position.y, position.z] {
        bytes.extend_from_slice(&axis.to_bits().to_be_bytes());
    }
    for angle in [-0.0_f32, 0.0] {
        bytes.extend_from_slice(&angle.to_bits().to_be_bytes());
    }
    for motion in [-0.0_f64, 0.0, 0.0] {
        bytes.extend_from_slice(&motion.to_bits().to_be_bytes());
    }
    bytes.extend_from_slice(&(-0.0_f32).to_bits().to_be_bytes());
    for value in [counters.age(), counters.pickup_delay(), counters.health()] {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
    bytes.extend_from_slice(&[0; 34]);
    bytes.extend_from_slice(&u32::try_from(stack.len()).unwrap().to_be_bytes());
    bytes.extend_from_slice(&stack);
    PersistentEntityRecord::new(
        ChunkPos::ORIGIN,
        EntityRecord::new(SchemaVersion::new(1), bytes).unwrap(),
    )
    .unwrap()
}

impl SimEnv {
    /// Seeds one genuine stored item and restores it into the shard; its
    /// publication group is acknowledged so the FIFO starts empty.
    pub async fn item(&mut self, n: u8, position: Vec3) -> EntityHandle {
        let key = PersistentEntityKey::new(scope().world(), scope().dimension(), item_uuid(n));
        let snapshot = self
            .store
            .load_owned_entity(key, generation().identity())
            .await
            .unwrap();
        let transfer = self.fresh_id();
        self.store
            .compare_exchange_owned_entity(
                EntityOwnershipCasRequest::new(
                    transfer,
                    snapshot.expectation().clone(),
                    EntityOwnershipProposal::put(item_record(n, 4, position)),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        let read = self
            .binding
            .read_entity(&self.store, item_uuid(n))
            .await
            .unwrap();
        self.shard
            .enqueue_owned_player_input(OwnedPlayerInput::restore_item(
                read,
                EntityId::new(1000 + i32::from(n)),
            ))
            .unwrap();
        let mut out = self.boundary();
        let refusals = out.take_refusals();
        assert!(
            refusals.is_empty(),
            "item restored: {:?}",
            refusals
                .iter()
                .map(|r| r.reason().to_string())
                .collect::<Vec<_>>()
        );
        self.ack_head();
        self.shard
            .entities()
            .unwrap()
            .handle_for_uuid(item_uuid(n))
            .unwrap()
    }

    /// Acknowledges the publication FIFO head (stand-in for NET-04.2b's
    /// complete group admission).
    pub fn ack_head(&mut self) {
        let ticket = self
            .shard
            .peek_item_publication()
            .unwrap()
            .unwrap()
            .ticket();
        self.shard.acknowledge_item_publication(&ticket).unwrap();
    }

    /// A genuine animated or plain pickup of item `n` by `player`; returns the
    /// outcome holding its work.
    pub async fn pickup(
        &mut self,
        player: &OwnedPlayerHandle,
        n: u8,
        animate: bool,
    ) -> PlayerOwnershipTickOutcome {
        let read = self
            .binding
            .read_pair(&self.store, player.player(), item_uuid(n))
            .await
            .unwrap();
        let item = self
            .shard
            .entities()
            .unwrap()
            .handle_for_uuid(item_uuid(n))
            .unwrap();
        let transfer = self.fresh_id();
        self.shard
            .enqueue_owned_player_input(OwnedPlayerInput::pickup(
                player.clone(),
                item,
                read,
                transfer,
                animate,
                false,
            ))
            .unwrap();
        let mut out = self.boundary();
        assert!(out.take_refusals().is_empty(), "pickup admitted");
        out
    }

    /// A genuine autosave admitted directly in the simulation (a pending of
    /// the player that is not the session's leave-save); returns its outcome.
    pub fn foreign_autosave(&mut self, player: &OwnedPlayerHandle) -> PlayerOwnershipTickOutcome {
        let transfer = self.fresh_id();
        self.shard
            .enqueue_owned_player_input(OwnedPlayerInput::autosave(player.clone(), transfer))
            .unwrap();
        let mut out = self.boundary();
        assert!(out.take_refusals().is_empty(), "autosave admitted");
        out
    }

    /// Executes every work of `out` against the genuine store and queues the
    /// completions (applied by the next boundary).
    pub async fn complete_all(&mut self, out: &mut PlayerOwnershipTickOutcome) -> usize {
        let works = out.take_work();
        let count = works.len();
        for work in works {
            let completion = work.execute(&self.store).await.unwrap();
            self.shard
                .enqueue_owned_player_input(OwnedPlayerInput::complete(completion))
                .unwrap();
        }
        count
    }
}

/// One driver iteration's observations.
#[derive(Debug, Default)]
pub struct Iteration {
    pub detached: Vec<NativeDetachOutcome>,
    pub changed: usize,
    pub stale_intents: usize,
    pub detached_intents: Vec<NativeRetentionToken>,
    pub refusals: usize,
}

/// Driver-like loop over one native registration.
pub struct Driver {
    pub router: SessionRouter,
    pub env: SimEnv,
    pub lane: NativeShardReceiver,
    /// Successfully applied normal boundaries (the contract's `applied`).
    pub applied: u64,
}

impl Driver {
    /// Drains the lane into the simulation, runs one normal boundary and
    /// dispatches its intents to the session the way §6.3 prescribes. The work
    /// stays in the returned outcome for the test to execute.
    pub fn iterate(&mut self) -> (PlayerOwnershipTickOutcome, Iteration) {
        while let Ok(input) = self.lane.try_recv() {
            let (_key, _class, input) = input.into_parts();
            self.env.shard.enqueue_owned_player_input(input).unwrap();
        }
        let mut out = self.env.boundary();
        self.applied += 1;
        let mut seen = Iteration {
            refusals: out.take_refusals().len(),
            ..Iteration::default()
        };
        for intent in out.intents() {
            match intent {
                OwnershipIntent::PlayerChanged { player, .. }
                | OwnershipIntent::Resync { player, .. } => {
                    let resync = matches!(intent, OwnershipIntent::Resync { .. });
                    if let Ok(view) = self.env.shard.connected_owned_player_view(player) {
                        let result = if resync {
                            self.router.native_resync(&view)
                        } else {
                            self.router.native_player_changed(&view)
                        };
                        result.unwrap();
                        seen.changed += 1;
                    } else if let Some(token) = self.router.native_detached_intent(player) {
                        seen.detached_intents.push(token);
                    } else {
                        seen.stale_intents += 1;
                    }
                }
                OwnershipIntent::PlayerDetached { player } => {
                    let shard = &self.env.shard;
                    let outcome = self
                        .router
                        .native_detached(*player, &mut |handle, probe| {
                            probe.submit(shard.collector_lifecycle_view(handle).ok())
                        })
                        .unwrap();
                    seen.detached.push(outcome);
                }
                _ => {}
            }
        }
        (out, seen)
    }

    /// Runs `n` driver iterations without new work.
    pub fn idle(&mut self, n: u64) {
        for _ in 0..n {
            let (mut out, _) = self.iterate();
            assert!(out.take_work().is_empty(), "idle boundary produced work");
        }
    }

    /// One `service_native_retention` step against the genuine status; a
    /// submit is enqueued into the simulation (an enqueue refusal is reported
    /// back with `native_retention_unsent`).
    pub fn service(&mut self, token: NativeRetentionToken, submit_allowed: bool) -> Serviced {
        let shard = &self.env.shard;
        let mut next = self.env.next_id_peek();
        let step = self
            .router
            .service_native_retention(
                token,
                self.applied,
                submit_allowed,
                &mut |handle| shard.detached_owned_player_status(handle),
                &mut || next.take(),
            )
            .unwrap();
        self.env.commit_ids(&next);
        match step {
            NativeRetentionStep::Submit(input) => {
                let (_key, _class, input) = input.into_parts();
                match self.env.shard.enqueue_owned_player_input(input) {
                    Ok(()) => Serviced::Submitted,
                    Err(refusal) => {
                        drop(refusal);
                        self.router.native_retention_unsent(token).unwrap();
                        Serviced::Unsent
                    }
                }
            }
            NativeRetentionStep::Waiting => Serviced::Waiting,
            NativeRetentionStep::SaveConfirmed => Serviced::Confirmed,
            NativeRetentionStep::Quarantined(q) => Serviced::Quarantined(q),
            NativeRetentionStep::Superseded => Serviced::Superseded,
            other => panic!("unexpected retention step {other:?}"),
        }
    }

    /// D5 with the genuine lifecycle view.
    pub fn retire(&mut self) -> ferrumc_session::NativeRetirementReport {
        let shard = &self.env.shard;
        self.router
            .retire_detached_native_sessions(&mut |handle, probe| {
                probe.submit(shard.collector_lifecycle_view(handle).ok())
            })
            .unwrap()
    }

    /// D7/D8 as one closure (Root decision 2(a)).
    pub fn acknowledge(&mut self) -> ferrumc_session::NativeAcknowledgeReport {
        let shard = &mut self.env.shard;
        self.router
            .acknowledge_retired_native_records(&mut |handle, probe| {
                probe
                    .submit(shard.collector_lifecycle_view(handle).ok())
                    .acknowledge(|ticket| shard.acknowledge_collector_lifecycle(ticket))
            })
            .unwrap()
    }

    /// §7.5 release through the session closure.
    pub fn release(
        &mut self,
        token: NativeRetentionToken,
    ) -> Result<(), ferrumc_session::CollectorSessionError> {
        let shard = &mut self.env.shard;
        self.router.release_native_retention(token, &mut |handle| {
            shard.release_detached_owned_player(handle)
        })
    }
}

/// Outcome of one [`Driver::service`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Serviced {
    Submitted,
    Unsent,
    Waiting,
    Confirmed,
    Quarantined(ferrumc_session::LeaveSaveQuarantine),
    Superseded,
}

/// A lazily drawn driver operation id for one service call.
pub struct NextId {
    base: u128,
    used: bool,
}

impl NextId {
    fn take(&mut self) -> OwnershipTransferId {
        assert!(!self.used, "one id per service step");
        self.used = true;
        OwnershipTransferId::from_bytes((self.base | (1_u128 << 120)).to_be_bytes())
    }
}

impl SimEnv {
    fn next_id_peek(&self) -> NextId {
        NextId {
            base: self.next_id + 1,
            used: false,
        }
    }

    fn commit_ids(&mut self, next: &NextId) {
        if next.used {
            self.next_id = next.base;
        }
    }
}

/// The handle of the first `PlayerAdmitted` intent, if any.
pub fn admitted_in(out: &PlayerOwnershipTickOutcome) -> Option<OwnedPlayerHandle> {
    out.intents().iter().find_map(|intent| match intent {
        OwnershipIntent::PlayerAdmitted { player, .. } => Some(player.clone()),
        _ => None,
    })
}

/// The id of a player for a short name.
pub fn pid(name: &str) -> PlayerId {
    PlayerId::offline(name)
}
