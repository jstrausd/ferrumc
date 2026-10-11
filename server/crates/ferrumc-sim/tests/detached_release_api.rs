//! SIM-03.4b public surface: storage identity re-exports (R2-A4) and the
//! authority-free detached release types, named only through `ferrumc_sim`.

use ferrumc_sim::{
    DetachedOwnedPlayerStatus, DetachedReleaseRefusal, OwnedPlayerHandle, OwnershipReceiptQuery,
    OwnershipTransferId, PlayerOwnershipError, ReleasedOwnedPlayer, SimShard,
};

// The re-exports are the storage types themselves, not wrappers: a consumer
// such as the session layer can pass them on without a storage dependency.
fn same_transfer_id(id: OwnershipTransferId) -> ferrumc_storage::OwnershipTransferId {
    id
}
fn same_query(query: OwnershipReceiptQuery) -> ferrumc_storage::OwnershipReceiptQuery {
    query
}

#[test]
fn reexported_identities_are_the_storage_types() {
    let id = OwnershipTransferId::from_bytes([7; 16]);
    assert_eq!(
        same_transfer_id(id),
        ferrumc_storage::OwnershipTransferId::from_bytes([7; 16])
    );
    let _: fn(OwnershipReceiptQuery) -> ferrumc_storage::OwnershipReceiptQuery = same_query;
}

#[test]
fn release_refusals_are_typed_player_ownership_errors() {
    let error: PlayerOwnershipError = DetachedReleaseRefusal::NotSaved.into();
    assert!(matches!(
        error,
        PlayerOwnershipError::DetachedRelease(DetachedReleaseRefusal::NotSaved)
    ));
    assert_eq!(
        error.to_string(),
        "detached player release refused: no confirmed save after detach"
    );
}

#[test]
fn status_and_receipt_expose_getters_only() {
    // Getter signatures are part of the contract used by the session closures.
    let _: fn(&DetachedOwnedPlayerStatus) -> Option<OwnershipTransferId> =
        DetachedOwnedPlayerStatus::saved_after_detach;
    let _: fn(
        &DetachedOwnedPlayerStatus,
    ) -> Option<(OwnershipTransferId, ferrumc_sim::OwnershipPendingStatus)> =
        DetachedOwnedPlayerStatus::blocking;
    let _: fn(&DetachedOwnedPlayerStatus) -> bool = DetachedOwnedPlayerStatus::dirty;
    let _: fn(&ReleasedOwnedPlayer) -> ferrumc_core::PlayerId = ReleasedOwnedPlayer::player;
    let _: fn(&ReleasedOwnedPlayer) -> usize = ReleasedOwnedPlayer::freed_player_bytes;
}

#[test]
fn shard_entry_points_take_only_the_original_handle() {
    // No caller query is accepted; status and readiness borrow the shard shared.
    let _: fn(
        &SimShard,
        &OwnedPlayerHandle,
    ) -> Result<DetachedOwnedPlayerStatus, PlayerOwnershipError> =
        SimShard::detached_owned_player_status;
    let _: fn(
        &mut SimShard,
        &OwnedPlayerHandle,
    ) -> Result<ReleasedOwnedPlayer, PlayerOwnershipError> =
        SimShard::release_detached_owned_player;
    let _: fn(&SimShard) -> Result<bool, PlayerOwnershipError> = SimShard::owned_stop_ready;
}
