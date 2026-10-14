//! Bounded completion ownership for serial, connection-pinned bucket exchanges.
use ferrumc_core::PlayerId;
use ferrumc_session::SessionError;
use ferrumc_sim::{BlockStateId, GameOutput};
use std::collections::BTreeMap;
use tokio::sync::oneshot;

pub(crate) type BucketReply = oneshot::Sender<Result<Option<BlockStateId>, SessionError>>;
pub(crate) struct BucketCompletions {
    pending: BTreeMap<PlayerId, (u64, i32, bool, BucketReply)>,
}
impl BucketCompletions {
    pub(crate) fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
        }
    }
    pub(crate) fn track(
        &mut self,
        player: PlayerId,
        token: u64,
        sequence: i32,
        fill: bool,
        reply: BucketReply,
    ) -> Result<(), BucketReply> {
        if token == 0 || self.pending.len() >= 4096 || self.pending.contains_key(&player) {
            return Err(reply);
        }
        self.pending.insert(player, (token, sequence, fill, reply));
        Ok(())
    }
    pub(crate) fn fail(&mut self, player: PlayerId, error: SessionError) {
        if let Some((_, _, _, reply)) = self.pending.remove(&player) {
            let _ = reply.send(Err(error));
        }
    }
    pub(crate) fn resolve(&mut self, output: &GameOutput) {
        match output {
            GameOutput::BucketUseResolved {
                player,
                token,
                sequence,
                fill,
                state,
                ..
            } if self
                .pending
                .get(player)
                .is_some_and(|(t, s, f, _)| t == token && s == sequence && f == fill) =>
            {
                if let Some((_, _, _, reply)) = self.pending.remove(player) {
                    let _ = reply.send(Ok(*state));
                }
            }
            GameOutput::PlayerDespawned { player } => {
                self.fail(*player, SessionError::UnknownPlayer { player: *player });
            }
            _ => {}
        }
    }
}
