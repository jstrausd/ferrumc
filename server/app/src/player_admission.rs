//! Bounded server-owned identity reservations protect player load/save ordering.
//! No lock spans I/O: a lease owns one UUID through admission and leave cleanup.

use std::collections::BTreeSet;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};

use ferrumc_core::PlayerId;

/// A classified refusal before any player record is loaded or defaults created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdmissionError {
    /// Another connection is loading, playing, or finishing this UUID.
    AlreadyReserved,
    /// A previous leave-save failed; restarting clears the reservation only.
    Quarantined,
    /// The server's bounded reservation capacity has been exhausted.
    AtCapacity,
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AlreadyReserved => "player UUID is already active or finishing leave",
            Self::Quarantined => "player UUID is quarantined after failed leave-save or cleanup",
            Self::AtCapacity => "player UUID reservation capacity is exhausted",
        })
    }
}

impl std::error::Error for AdmissionError {}

#[derive(Debug, Default)]
struct Identities {
    reserved: BTreeSet<PlayerId>,
    quarantined: BTreeSet<PlayerId>,
}

/// At most the configured connection ceiling of reserved or quarantined UUIDs.
#[derive(Debug)]
pub(crate) struct PlayerAdmissions {
    identities: Mutex<Identities>,
    capacity: usize,
}

impl PlayerAdmissions {
    /// Creates one registry per server; no process-global admission state exists.
    pub(crate) fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            identities: Mutex::new(Identities::default()),
            capacity,
        })
    }

    fn lock(&self) -> MutexGuard<'_, Identities> {
        self.identities
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Claims the canonical UUID before load and returns its lifetime guard.
    pub(crate) fn reserve(
        self: &Arc<Self>,
        player: PlayerId,
    ) -> Result<PlayerReservation, AdmissionError> {
        let mut identities = self.lock();
        if identities.quarantined.contains(&player) {
            return Err(AdmissionError::Quarantined);
        }
        if identities.reserved.contains(&player) {
            return Err(AdmissionError::AlreadyReserved);
        }
        if identities.reserved.len() + identities.quarantined.len() >= self.capacity {
            return Err(AdmissionError::AtCapacity);
        }
        identities.reserved.insert(player);
        Ok(PlayerReservation {
            registry: Arc::clone(self),
            player,
            quarantined: false,
        })
    }
}

/// Owns one reserved UUID until its complete connection lifecycle has finished.
#[derive(Debug)]
pub(crate) struct PlayerReservation {
    registry: Arc<PlayerAdmissions>,
    player: PlayerId,
    quarantined: bool,
}

impl PlayerReservation {
    /// Prevents stale-record admission after a failed save/leave. This bounded
    /// quarantine survives this lease's drop until the owning server is dropped;
    /// it cannot repair a failed durable write or make a restart safe by itself.
    pub(crate) fn quarantine(&mut self) {
        let mut identities = self.registry.lock();
        identities.reserved.remove(&self.player);
        identities.quarantined.insert(self.player);
        self.quarantined = true;
    }
}

impl Drop for PlayerReservation {
    fn drop(&mut self) {
        if !self.quarantined {
            self.registry.lock().reserved.remove(&self.player);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot;

    #[test]
    fn claims_are_uuid_specific_bounded_and_released_on_failed_admission() {
        let registry = PlayerAdmissions::new(1);
        let player = PlayerId::offline("First");
        let first = registry.reserve(player).expect("first claim");
        assert_eq!(
            registry.reserve(player).expect_err("duplicate"),
            AdmissionError::AlreadyReserved
        );
        assert_eq!(
            registry
                .reserve(PlayerId::offline("Second"))
                .expect_err("full"),
            AdmissionError::AtCapacity
        );
        drop(first);
        let admitted = registry
            .reserve(player)
            .expect("failed pre-join admission releases claim");
        drop(admitted);
    }

    #[test]
    fn failed_save_quarantine_retains_one_bounded_slot_until_server_restart() {
        let registry = PlayerAdmissions::new(1);
        let player = PlayerId::offline("FailedSave");
        let mut lease = registry.reserve(player).expect("claim");
        lease.quarantine();
        lease.quarantine();
        drop(lease);
        assert_eq!(
            registry.reserve(player).expect_err("unsafe reload"),
            AdmissionError::Quarantined
        );
        assert_eq!(
            registry
                .reserve(PlayerId::offline("Other"))
                .expect_err("bounded quarantine"),
            AdmissionError::AtCapacity
        );
        assert!(
            PlayerAdmissions::new(1).reserve(player).is_ok(),
            "restart clears identity state only"
        );
    }

    #[tokio::test]
    async fn delayed_save_and_leave_both_keep_fast_rejoin_outside_record_loading() {
        let registry = PlayerAdmissions::new(1);
        let player = PlayerId::offline("FastRejoin");
        let lease = registry.reserve(player).expect("old connection claim");
        let (saved, save_complete) = oneshot::channel::<()>();
        let (left, leave_complete) = oneshot::channel::<()>();
        let (saving, save_started) = oneshot::channel::<()>();
        let (leaving, leave_started) = oneshot::channel::<()>();
        let old_connection = tokio::spawn(async move {
            saving.send(()).expect("save barrier");
            save_complete.await.expect("durable save completion");
            leaving.send(()).expect("leave barrier");
            leave_complete.await.expect("driver leave acknowledgment");
            drop(lease);
        });
        save_started.await.expect("save entered");
        assert_eq!(
            registry.reserve(player).expect_err("save pending"),
            AdmissionError::AlreadyReserved
        );
        saved.send(()).expect("commit save");
        leave_started.await.expect("leave entered");
        assert_eq!(
            registry.reserve(player).expect_err("leave pending"),
            AdmissionError::AlreadyReserved
        );
        left.send(()).expect("acknowledge leave");
        old_connection.await.expect("old lifecycle completed");
        assert!(
            registry.reserve(player).is_ok(),
            "only fresh post-save data may now load"
        );
    }
}
