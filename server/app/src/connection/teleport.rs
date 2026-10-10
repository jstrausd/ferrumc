//! The bounded, connection-owned teleport acknowledgement contract.

use std::fmt;

use ferrumc_proto::generated::play::SynchronizePlayerPosition;

/// The latest server-issued ID and at most one outstanding acknowledgement.
///
/// Server positions are authoritative before an acknowledgement. This state
/// gates later client positions; it owns no world data or additional timer.
#[derive(Debug, Default)]
pub(super) struct PendingTeleport {
    last_id: i32,
    pending: bool,
}

impl PendingTeleport {
    /// Gives this connection's next ID to an already authoritative sync.
    /// A newer sync replaces the pending one without retaining an ID history.
    pub(super) fn issue(&mut self, sync: &SynchronizePlayerPosition) -> SynchronizePlayerPosition {
        self.last_id = if self.last_id == i32::MAX - 1 {
            0
        } else {
            self.last_id + 1
        };
        self.pending = true;
        SynchronizePlayerPosition::new(
            self.last_id,
            sync.x(),
            sync.y(),
            sync.z(),
            sync.dx(),
            sync.dy(),
            sync.dz(),
            sync.yaw(),
            sync.pitch(),
            sync.flags(),
        )
    }

    /// Matches Paper's current-ID check: stale IDs are ignored, whereas a replay
    /// of the current ID after its acknowledgement is a protocol violation.
    pub(super) fn confirm(&mut self, id: i32) -> Result<(), TeleportConfirmationError> {
        if id != self.last_id {
            return Ok(());
        }
        if !self.pending {
            return Err(TeleportConfirmationError { id });
        }
        self.pending = false;
        Ok(())
    }

    /// Whether client positions must still be withheld.
    pub(super) fn is_pending(&self) -> bool {
        self.pending
    }
}

/// A confirmation of the current ID without an outstanding server teleport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TeleportConfirmationError {
    id: i32,
}

impl fmt::Display for TeleportConfirmationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid teleport confirmation {}: no pending teleport",
            self.id
        )
    }
}

impl std::error::Error for TeleportConfirmationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sync() -> SynchronizePlayerPosition {
        SynchronizePlayerPosition::new(991, 8.0, 64.0, 8.0, 1.0, 2.0, 3.0, 31.0, -17.0, 0)
    }

    #[test]
    fn newest_id_supersedes_and_only_one_matching_echo_releases() {
        let mut pending = PendingTeleport::default();
        let first = pending.issue(&sync());
        let second = pending.issue(&sync());
        assert_eq!(first.teleport_id(), 1);
        assert_eq!(second.teleport_id(), 2);
        for stale in [i32::MIN, -1, 0, 1, 3, i32::MAX] {
            pending.confirm(stale).expect("stale ID is ignored");
            assert!(pending.is_pending());
        }
        pending.confirm(2).expect("newest ID matches");
        assert!(!pending.is_pending());
        pending.confirm(1).expect("old ID stays harmless");
        assert_eq!(pending.confirm(2), Err(TeleportConfirmationError { id: 2 }));
    }

    #[test]
    fn id_wrap_matches_paper_and_preserves_every_other_wire_field() {
        let mut pending = PendingTeleport {
            last_id: i32::MAX - 2,
            pending: false,
        };
        assert_eq!(pending.issue(&sync()).teleport_id(), i32::MAX - 1);
        let wrapped = pending.issue(&sync());
        assert_eq!(wrapped.teleport_id(), 0);
        let expected =
            SynchronizePlayerPosition::new(0, 8.0, 64.0, 8.0, 1.0, 2.0, 3.0, 31.0, -17.0, 0);
        assert_eq!(wrapped, expected);
        pending.confirm(0).expect("zero is a valid wrapped ID");
        assert_eq!(pending.issue(&sync()).teleport_id(), 1);
    }

    #[test]
    fn no_pending_current_id_fails_and_other_ids_are_ignored() {
        let mut pending = PendingTeleport::default();
        pending.confirm(-1).expect("unrelated unsolicited ID");
        assert_eq!(pending.confirm(0), Err(TeleportConfirmationError { id: 0 }));
    }
}
