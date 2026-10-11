# Invariants: ferrumc-session

> Rules that must hold for all code in this crate. Violating these is a bug.

## General

- No `unwrap()` or `expect()` outside `#[cfg(test)]`.
- No unbounded channels or allocations from untrusted input.
- All public items have rustdoc.
- Error types classify the failure mode.

## Crate-Specific

- The router never holds a `SimShard`, a `Chunk`, a socket, or a DB handle.
  For **legacy** coverage it exchanges only `GameInput`/`GameOutput`/
  `ClientboundPlayPacket` messages. For **native** coverage (NET-04.2e1) it
  additionally sends `NativeShardInput` over the registration's bounded native
  lane; every native gameplay/disconnect input is built from the genuine
  `OwnedPlayerHandle` stored at `admit_native_join`, never from a caller value.
- Legacy and native coverage exclude each other per world/dimension
  (`EndpointKindConflict`). A legacy input or lease operation that resolves to a
  native endpoint is refused with `NativeEndpointRequiresSession`, input intact;
  no UUID-only input is translated for a native owner.
- One checked router allocator issues every remote entity wire id from
  `[2, i32::MAX]`; `i32::MAX` is issued once, then the allocator is exhausted.
  It never wraps or reuses ids. Exhaustion (and the opt-in session limit) is
  refused before any slow-viewer disconnect, `PlayerJoin`, or native
  reservation; native reservations burn their id and incarnation even when
  later cancelled. Legacy ids are allocated at the historical point, so the
  non-exhausted legacy behaviour is unchanged.
- `PlayerSessionKey` is sealed (weak router identity, player, checked
  `NonZeroU64` incarnation). Keys of another router or incarnation never address
  a session (`StaleSession`). The key confers no authority.
- The native lane is bounded (router shard-input capacity, at most 1024). Data
  stops before the control reserve; disconnects may use it. A full lane turns
  active-session data into a controlled session leave; a disconnect that cannot
  be queued (full/closed lane, suspended endpoint) is retained once per session
  and retried (`retry_native_disconnects`) or sent first after revalidation.
- Movement coalescing (Root R3-A2/F4): a movement lane entry shares one
  per-session `Arc<std::sync::Mutex<…>>` cell between router and receiver. The
  lock is held only for O(1) work without allocation, never across `.await`,
  never together with another lock, and the cell is owned by exactly one
  session entry (no global cell). Any non-movement input, a teleport
  confirmation, a resync or region correction, a replacement or a leave closes
  the cell, so movement is never reordered across them. A poisoned cell is
  reported as `NativeLaneRecv::MovementCellPoisoned` and never merged again.
- Native movement is pre-checked with the simulation's own derivation
  (`EntityTransform::try_new(position, 0, 0)?.chunk()` inside the home region);
  a refusal queues a mandatory sync to the last genuine pose and no lane item.
- Endpoint replacement suspends every native session of the registration.
  Only `revalidate_native_endpoints` rebinds them: the session passes each
  stored handle to the driver's closure with a single-use probe, and only the
  probe's `submit` produces a verdict (Root R3-A1). The driver holds no handle.
- NET-04.2e1 has no retention record, token, lineage, leave-save, adoption or
  retirement (NET-04.2e2). `disconnect_native` sets the entry `Leaving`, closes
  its own outbound and queues/retains exactly one disconnect; the entry stays
  in the table (counted, blocking its UUID and unregistration) for e2 to convert
  in place. Native sessions do not see each other until e2 audience/reconcile.
- All transport is bounded `tokio::sync::mpsc`; routing uses non-blocking
  `try_send`, so the router never blocks the tick loop. Ordinary data stops at a
  fixed reserved tail for join/leave/reject control traffic. A rejected input is
  returned with its typed delivery policy and classified `SessionError`, never
  silently dropped.
- A lifecycle leave rejected during a slow-client cascade remains represented in
  a player-bounded pending-disconnect set until an explicit retry accepts it;
  cascade errors are never reduced to a log line or discarded result.
- The player<->shard mapping is the single source of truth for player location.
  `disconnect_player` must enqueue `PlayerLeave` before removing the mapping; if
  the control lane rejects the leave, the mapping remains available for retry or
  explicit overload termination.
- Logical `ShardId` targets and runtime endpoints are distinct. Directory
  resolution is scoped by `WorldId` + `DimensionId`, prefers exact coverage over
  a world-covering fallback, and never falls through after a selected endpoint
  reports full or closed.
- Directory registration is atomic and lease-validated. Duplicate insertion
  cannot overwrite an endpoint; replace/remove requires the current generation;
  generations and registration-lineage ids are never reused after removal
  within a directory instance.
- A player binding retains one coverage slot, endpoint home, and registration
  lineage. An authorized sender rotation preserves that lineage and immediately
  serves existing sessions. Unregister/re-register creates a new lineage and
  cannot silently retarget them.
- Translation (`net_event_to_input`, `output_to_clientbound`,
  `shard_for_position`) is pure: no channels, maps, or I/O.

## Dependencies

- Production code depends on simulation types only through `ferrumc-sim`; it
  never depends on raw storage or the raw world model. Exception (Root decision
  NET-04.2e1 F2): `ferrumc-storage` and `ferrumc-world` are **dev-dependencies**
  so contract tests can create genuine memory/redb admissions. A source-scan
  test refuses any mention of them outside `#[cfg(test)]` modules and `tests/`.
