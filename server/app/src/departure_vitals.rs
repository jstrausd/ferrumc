//! Read-only final observations outlive a forced Session removal until Leave ack.

use std::collections::BTreeMap;

use ferrumc_core::PlayerId;
use ferrumc_sim::{GameOutput, SimShard, VitalsSnapshot};

// The configuration admits at most 1024 concurrent connection leases. A stalled
// teardown retains its observation and refuses a new registration at this cap.
const MAX_OBSERVATIONS: usize = 1024;

#[derive(Default)]
pub(crate) struct DepartureVitals {
    observations: BTreeMap<PlayerId, Option<VitalsSnapshot>>,
}

impl DepartureVitals {
    /// Reserves the connection lifetime without inventing initial health.
    pub(crate) fn register(&mut self, player: PlayerId) -> bool {
        if !self.observations.contains_key(&player) && self.observations.len() == MAX_OBSERVATIONS {
            return false;
        }
        self.observations.insert(player, None);
        true
    }

    /// Captures actual owner state, including timers omitted from health packets.
    pub(crate) fn capture(&mut self, shard: &SimShard) {
        for (player, observation) in &mut self.observations {
            if let Some(snapshot) = shard.player_vitals(*player) {
                *observation = Some(snapshot);
            }
        }
    }

    /// Preserves changes when an accepted Leave removes the actor in the same tick.
    pub(crate) fn observe_outputs(&mut self, outputs: &[GameOutput]) {
        for output in outputs {
            if let GameOutput::PlayerVitalsChanged { player, snapshot }
            | GameOutput::PlayerDepartureVitals { player, snapshot } = output
            {
                if let Some(observation) = self.observations.get_mut(player) {
                    *observation = Some(*snapshot);
                }
            }
        }
    }

    /// Current owner state wins; departure only exposes a genuine past observation.
    pub(crate) fn read(&self, shard: &SimShard, player: PlayerId) -> Option<VitalsSnapshot> {
        shard
            .player_vitals(player)
            .or_else(|| self.observations.get(&player).copied().flatten())
    }

    /// Only completed connection teardown retires this read-only lifetime slot.
    pub(crate) fn remove(&mut self, player: PlayerId) {
        self.observations.remove(&player);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrumc_math::{ShardPos, Vec3};
    use ferrumc_sim::{Difficulty, GameInput};

    #[test]
    fn same_tick_change_and_leave_retains_actual_snapshot_without_default_or_reuse() {
        let player = PlayerId::offline("same-boundary-departure");
        let mut shard = SimShard::new(ShardPos::new(0, 0));
        shard.enable_survival(Difficulty::Normal, false).unwrap();
        let mut observations = DepartureVitals::default();
        assert!(observations.register(player));
        assert!(observations.read(&shard, player).is_none());
        let snapshot = VitalsSnapshot::new(7.0, 12, 2.0, 3.0)
            .unwrap()
            .with_timers(37, 9, 5, 4.0, 13)
            .unwrap();
        for input in [
            GameInput::PlayerJoin {
                player,
                position: Vec3::new(8.0, 64.0, 8.0),
            },
            GameInput::RestoreVitals { player, snapshot },
            GameInput::PlayerLeave { player },
        ] {
            shard.enqueue(input).unwrap();
        }
        let outputs = shard.run_tick();
        observations.observe_outputs(&outputs);
        observations.capture(&shard);
        assert!(shard.player_vitals(player).is_none());
        assert_eq!(observations.read(&shard, player), Some(snapshot));
        observations.remove(player);
        assert!(observations.read(&shard, player).is_none());
        assert!(observations.register(player));
        assert!(observations.read(&shard, player).is_none());
    }

    #[test]
    fn stalled_departures_are_bounded_and_completed_leave_releases_capacity() {
        let mut observations = DepartureVitals::default();
        let players: Vec<_> = (0..MAX_OBSERVATIONS)
            .map(|id| PlayerId::offline(&format!("departure-{id}")))
            .collect();
        for &player in &players {
            assert!(observations.register(player));
        }
        let next = PlayerId::offline("next-departure");
        assert!(!observations.register(next));
        observations.remove(players[0]);
        assert!(observations.register(next));
    }

    #[tokio::test]
    async fn silent_sprint_exhaustion_is_captured_before_same_boundary_leave() {
        use ferrumc_math::ChunkPos;
        use ferrumc_sim::{ChunkTicket, TicketReason};
        use ferrumc_storage::InMemoryStore;
        use ferrumc_world::FlatWorldGenerator;

        let mut shard = SimShard::new(ShardPos::new(0, 0));
        shard
            .loaded_chunks_mut()
            .acquire(
                &InMemoryStore::new(),
                &FlatWorldGenerator::new(),
                ChunkPos::ORIGIN,
                ChunkTicket::of(TicketReason::Player),
            )
            .await
            .unwrap();
        shard.enable_survival(Difficulty::Normal, false).unwrap();
        let player = PlayerId::offline("silent-departure");
        shard
            .enqueue(GameInput::PlayerJoin {
                player,
                position: Vec3::new(8.0, 64.0, 8.0),
            })
            .unwrap();
        shard.run_tick();
        let mut observations = DepartureVitals::default();
        assert!(observations.register(player));
        observations.capture(&shard);
        for input in [
            GameInput::FallMovement {
                player,
                position: Vec3::new(12.0, 64.0, 8.0),
                on_ground: true,
                sprinting: true,
            },
            GameInput::PlayerLeave { player },
        ] {
            shard.enqueue(input).unwrap();
        }
        let outputs = shard.run_tick();
        assert!(!outputs
            .iter()
            .any(|output| matches!(output, GameOutput::PlayerVitalsChanged { .. })));
        let departure = outputs
            .iter()
            .find_map(|output| match output {
                GameOutput::PlayerDepartureVitals { snapshot, .. } => Some(*snapshot),
                _ => None,
            })
            .unwrap();
        assert_eq!(departure.exhaustion().to_bits(), 0.4_f32.to_bits());
        for output in &outputs {
            if matches!(output, GameOutput::PlayerDepartureVitals { .. }) {
                assert!(ferrumc_session::output_to_clientbound(output).is_none());
            }
        }
        assert!(shard.player_vitals(player).is_none());
        observations.observe_outputs(&outputs);
        observations.capture(&shard);
        assert_eq!(observations.read(&shard, player), Some(departure));
    }
}
