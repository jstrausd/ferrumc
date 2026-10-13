//! One bounded completion owner per player's serial placement handler.
//!
//! Admission and previews are not mutation outcomes. Only the simulation's
//! dedicated placement result can release a successful after-hook notification.

use std::collections::BTreeMap;

use ferrumc_core::PlayerId;
use ferrumc_math::BlockPos;
use ferrumc_session::SessionError;
use ferrumc_sim::{BlockStateId, GameOutput};
use tokio::sync::oneshot;

pub(crate) type PlacementReply =
    oneshot::Sender<Result<Option<(BlockPos, BlockStateId)>, SessionError>>;

/// Explicit ceiling independent of externally configurable connection counts.
pub(crate) const MAX_PENDING_PLACEMENTS: usize = 4096;

pub(crate) struct PlacementCompletions {
    pending: BTreeMap<PlayerId, (i32, PlacementReply)>,
}

impl PlacementCompletions {
    pub(crate) fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
        }
    }

    /// Reserve before routing; refusal preserves the caller's completion owner.
    pub(crate) fn track(
        &mut self,
        player: PlayerId,
        sequence: i32,
        reply: PlacementReply,
    ) -> Result<(), PlacementReply> {
        if self.pending.len() >= MAX_PENDING_PLACEMENTS || self.pending.contains_key(&player) {
            return Err(reply);
        }
        self.pending.insert(player, (sequence, reply));
        Ok(())
    }

    pub(crate) fn fail(&mut self, player: PlayerId, error: SessionError) {
        if let Some((_, reply)) = self.pending.remove(&player) {
            let _ = reply.send(Err(error));
        }
    }

    pub(crate) fn resolve(&mut self, output: &GameOutput) {
        match output {
            GameOutput::PlayerPlacementResolved {
                player,
                sequence,
                position,
                state,
            } if self
                .pending
                .get(player)
                .is_some_and(|(seq, _)| seq == sequence) =>
            {
                if let Some((_, reply)) = self.pending.remove(player) {
                    let _ = reply.send(Ok(state.map(|state| (*position, state))));
                }
            }
            GameOutput::PlayerDespawned { player } => {
                self.fail(*player, SessionError::UnknownPlayer { player: *player });
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrumc_sim::MutationCause;

    #[test]
    fn broadcast_or_reused_action_sequence_cannot_fake_a_placement_result() {
        let player = PlayerId::offline("placing");
        let mut completions = PlacementCompletions::new();
        let (tx, mut rx) = oneshot::channel();
        assert!(completions.track(player, 7, tx).is_ok());
        completions.resolve(&GameOutput::BlockChanged {
            position: BlockPos::new(8, 64, 8),
            state: BlockStateId::AIR,
            sequence: 7,
            cause: MutationCause::PlayerCreative { player },
        });
        completions.resolve(&GameOutput::PlayerPlacementResolved {
            player,
            position: BlockPos::new(8, 64, 8),
            sequence: 6,
            state: Some(BlockStateId::new(1)),
        });
        assert!(matches!(
            rx.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));
        completions.resolve(&GameOutput::PlayerPlacementResolved {
            player,
            position: BlockPos::new(8, 64, 8),
            sequence: 7,
            state: Some(BlockStateId::new(136)),
        });
        assert_eq!(
            rx.try_recv(),
            Ok(Ok(Some((BlockPos::new(8, 64, 8), BlockStateId::new(136)))))
        );
        assert!(completions.pending.is_empty());
    }

    #[test]
    fn collision_refusal_and_departure_release_completion_without_success() {
        let player = PlayerId::offline("refused");
        let mut completions = PlacementCompletions::new();
        let (tx, mut rx) = oneshot::channel();
        assert!(completions.track(player, 9, tx).is_ok());
        let (duplicate, _) = oneshot::channel();
        assert!(completions.track(player, 10, duplicate).is_err());
        completions.resolve(&GameOutput::PlayerPlacementResolved {
            player,
            position: BlockPos::new(8, 64, 8),
            sequence: 9,
            state: None,
        });
        assert_eq!(rx.try_recv(), Ok(Ok(None)));

        let (tx, mut rx) = oneshot::channel();
        assert!(completions.track(player, 10, tx).is_ok());
        completions.resolve(&GameOutput::PlayerDespawned { player });
        assert_eq!(
            rx.try_recv(),
            Ok(Err(SessionError::UnknownPlayer { player }))
        );
        assert!(completions.pending.is_empty());
    }
}
