//! One exact owner receipt per serial connection-owned attack transaction.
use ferrumc_core::PlayerId;
use ferrumc_sim::GameOutput;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::oneshot;

pub(crate) type AttackReply = oneshot::Sender<bool>;
pub(crate) type ResolvedAttack = (
    PlayerId,
    Arc<crate::combat_wear::CombatWearSnapshot>,
    f32,
    tokio::sync::mpsc::OwnedPermit<crate::gameplay::GameplayEffect>,
);
pub(crate) struct PendingAttack {
    pub(crate) target: PlayerId,
    pub(crate) armor: Arc<crate::combat_wear::CombatWearSnapshot>,
    pub(crate) reply: AttackReply,
    pub(crate) effect: tokio::sync::mpsc::OwnedPermit<crate::gameplay::GameplayEffect>,
}

pub(crate) struct AttackCompletions {
    pending: BTreeMap<PlayerId, (u64, PendingAttack)>,
    bytes: usize,
}
impl AttackCompletions {
    pub(crate) fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            bytes: 0,
        }
    }
    pub(crate) fn track(
        &mut self,
        player: PlayerId,
        token: u64,
        pending: PendingAttack,
    ) -> Result<(), PendingAttack> {
        let charge = pending.armor.encoded_bytes();
        if token == 0
            || self.pending.len() >= 4096
            || self.pending.contains_key(&player)
            || self.bytes.saturating_add(charge) > 8 * 1024 * 1024
        {
            return Err(pending);
        }
        self.bytes += charge;
        self.pending.insert(player, (token, pending));
        Ok(())
    }
    pub(crate) fn matches(&self, player: PlayerId, target: PlayerId, token: u64) -> bool {
        self.pending
            .get(&player)
            .is_some_and(|(t, p)| *t == token && p.target == target)
    }
    pub(crate) fn fail(&mut self, player: PlayerId) -> Option<PlayerId> {
        if let Some((_, pending)) = self.pending.remove(&player) {
            self.bytes -= pending.armor.encoded_bytes();
            let _ = pending.reply.send(false);
            return Some(pending.target);
        }
        None
    }
    /// Returns the exact victim's immutable witness only on positive real damage.
    pub(crate) fn resolve(&mut self, output: &GameOutput) -> Option<ResolvedAttack> {
        match output {
            GameOutput::PlayerAttackResolved {
                player,
                target,
                token,
                accepted,
                raw_damage,
                ..
            } if self
                .pending
                .get(player)
                .is_some_and(|(identity, p)| identity == token && p.target == *target) =>
            {
                let (_, pending) = self.pending.remove(player)?;
                self.bytes -= pending.armor.encoded_bytes();
                let _ = pending.reply.send(*accepted);
                accepted.then_some((*target, pending.armor, *raw_damage, pending.effect))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrumc_core::GameMode;
    use ferrumc_items::ItemStack;

    fn pending(
        target: PlayerId,
    ) -> (
        PendingAttack,
        oneshot::Receiver<bool>,
        tokio::sync::mpsc::Receiver<crate::gameplay::GameplayEffect>,
    ) {
        let inventory = crate::inventory::PlayerInventory::from_persisted(
            GameMode::Survival,
            0,
            std::array::from_fn(|_| ItemStack::empty()),
            ItemStack::empty(),
        );
        let armor = Arc::new(inventory.combat_snapshot().unwrap());
        let (reply, outcome) = oneshot::channel();
        let (sender, effects) = tokio::sync::mpsc::channel(1);
        let effect = sender.try_reserve_owned().unwrap();
        (
            PendingAttack {
                target,
                armor,
                reply,
                effect,
            },
            outcome,
            effects,
        )
    }

    #[test]
    fn only_matching_receipt_transfers_reserved_publication_and_releases_evidence() {
        let player = PlayerId::offline("attacker");
        let target = PlayerId::offline("victim");
        let mut completions = AttackCompletions::new();
        let (pending, mut outcome, mut effects) = pending(target);
        assert!(completions.track(player, 7, pending).is_ok());
        let receipt = |token| GameOutput::PlayerAttackResolved {
            player,
            target,
            token,
            accepted: true,
            damage: 0.96,
            raw_damage: 4.0,
        };
        assert!(completions.resolve(&receipt(6)).is_none());
        assert!(outcome.try_recv().is_err());
        assert!(completions.matches(player, target, 7));
        let (actual, snapshot, raw_damage, permit) = completions.resolve(&receipt(7)).unwrap();
        assert_eq!(actual, target);
        assert!(outcome.try_recv().unwrap());
        let _ = permit.send(crate::gameplay::GameplayEffect::ArmorHurt {
            snapshot,
            raw_damage,
        });
        assert!(matches!(
            effects.try_recv().unwrap(),
            crate::gameplay::GameplayEffect::ArmorHurt {
                raw_damage: 4.0,
                ..
            }
        ));
        assert_eq!(completions.bytes, 0);
        assert!(completions.resolve(&receipt(7)).is_none());
    }

    #[test]
    fn cancellation_returns_the_exact_target_and_frees_all_reserved_capacity() {
        let player = PlayerId::offline("attacker");
        let target = PlayerId::offline("victim");
        let mut completions = AttackCompletions::new();
        let (pending, mut outcome, mut effects) = pending(target);
        assert!(completions.track(player, 1, pending).is_ok());
        assert_eq!(completions.fail(player), Some(target));
        assert!(!outcome.try_recv().unwrap());
        assert!(effects.try_recv().is_err());
        assert_eq!(completions.bytes, 0);
        assert_eq!(completions.fail(player), None);
    }
}
