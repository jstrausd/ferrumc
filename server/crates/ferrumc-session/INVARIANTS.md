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
  `ClientboundPlayPacket` messages. For **native** coverage (NET-04.2e1/e2) it
  additionally sends `NativeShardInput` over the registration's bounded native
  lane; every native gameplay, disconnect and leave-save input is built from
  the genuine `OwnedPlayerHandle` stored at `admit_native_join` or
  `adopt_unbound_native_admission`, never from a caller value.
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
- Take sequence (§7.3 amendment V1, Root): each lane generation counts the
  items the router (sole sender) put in and the items the receiver took out
  (two monotonic atomics in the lane identity, no lock). A sent disconnect
  keeps its lane position; it counts as taken once the take count reaches it.
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
- One unified native session table (NET-04.2e2, A1/R3-3): a reservation
  fallibly allocates the single entry (reusing a freed slot first) that the
  active session, its retention record and a G3 adoption later take over in
  place; no later transition allocates or can fail on capacity. Reservations,
  sessions, records and poisoned entries all count against the session limit.
- `disconnect_native` (D2) returns an opaque `NativeRetentionToken` (player plus
  a checked, never reused router-local lineage). The driver holds only tokens;
  the record keeps the original admission handle until release. The session is
  the single producer of the leave-save autosave, of an adopted admission's
  disconnect and of the release call (B2).
- Detach (D3), retirement (D5), lifecycle acknowledgement (D7/D8) and the
  unbound retirement read the genuine `CollectorLifecycleView` only inside the
  driver's closure through a single-use `NativeLifecycleProbe`; the verdict
  carries scalar facts and, for D7/D8, the owned ticket that
  `NativeLifecycleVerdict::acknowledge` lends exactly once (Root R3-A1 form).
  A verdict of another probe proves nothing.
- The leave-save state machine (§7.3, R2-A1) counts only on applied normal
  boundaries: at most 8 attempts, 8 disconnect resends (§7.3 amendment V1,
  Root: a resend counts only after the previous disconnect was taken from the
  lane, or submitted, and a normal boundary followed while the player stayed
  connected; a disconnect queued behind a backlog never counts), 20-boundary
  resubmit, one attempt per 100 boundaries of an unconfirmed own pending, a
  foreign blocking pending waits without an attempt (uncertain: per 100;
  prepared, in-flight or needs-refresh: per 100 after 1200). Stale and foreign
  handles, exhausted attempts and quarantined pendings end in a typed
  `LeaveSaveQuarantine`; snapshot/uninitialized status is a wiring break
  (`NativeStatusInvariant`). A later genuine confirmation still allows release.
  The stop phase A final save renews one attempt per quarantined record once;
  for a `DisconnectNotApplied` record whose player is still connected the next
  step returns it to that quarantine without an attempt or resend, never a
  `NativeStatusInvariant` (F1, round 2).
- A refused revalidation never rebinds: the stored handle's disconnect uses the
  current lane, also after every further replacement of the registration (F2,
  round 2), where the simulation applies it or refuses it (then the record is
  quarantined, never resent). A session revalidated as detached retains no
  disconnect, so `retry_native_disconnects` reaches zero (A2). After the
  confirmed detach fact, a control lane input is no longer live (A4).
- Release (§7.5) only for a retired, lifecycle-acknowledged record with a
  confirmed leave-save, through the driver's closure calling
  `release_detached_owned_player` with the lent original handle. A genuine
  rejoin supersedes the retired record (no leave-save, no release; removed after
  its acknowledgement).
- Native sessions track each other: a viewer's baseline entry per subject
  session (keyed by its incarnation) is created with a mandatory tab-list add,
  spawned (mandatory) while in view, despawned (mandatory, tab entry kept) when
  out of view, and removed with `EntityRemove` then `TabRemove` at D5. Every
  genuine pose update reconciles both directions; work, scratch, per-viewer and
  total baselines and the subject's own outbound are admitted before the first
  send. A viewer that cannot take a mandatory packet is set leaving.
- The pose producer (A3) takes position, look and mode only from a genuine
  `ConnectedOwnedPlayerView` of the stored admission; corrections always use the
  latest such pose, and a pose that differs from an outstanding sync while no
  newer client movement was routed gets one follow-up sync. Router syncs carry
  no teleport id; the connection stamps them (app `PendingTeleport`).
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
