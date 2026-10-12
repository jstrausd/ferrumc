# NET-04.2d — Root decisions on contract acceptance

Root, 11.10.2026. The concrete native driver and endpoint contract
[NET-04_2d_NATIVE_DRIVER_ENDPOINT.md](NET-04_2d_NATIVE_DRIVER_ENDPOINT.md)
(SHA-256 `e0530e0a`, design freeze `fe8c3207`) is independently
CLEAN_CONTRACT after three review rounds:
[round 1](NET-04_2d_NATIVE_DRIVER_ENDPOINT_REVIEW.md) (FINDINGS B1–B3),
[round 2](NET-04_2d_NATIVE_DRIVER_ENDPOINT_REVIEW_ROUND2.md) (FINDINGS N1–N3),
[round 3](NET-04_2d_NATIVE_DRIVER_ENDPOINT_REVIEW_ROUND3.md) (CLEAN_CONTRACT,
conditions R3-A1–A6). The round-3 reviewer reproduced all 202 anchors and
independently recomputed the 4096-item stop walkthrough.

The decisions below are binding for the named runtime tickets and take
precedence over the contract text where they differ. No runtime, Survival,
plugin or performance acceptance follows from this contract.

## R3-A1 — revalidation signature (NET-04.2e1)

The D7/D8 closure signature of the ".2c amendment R2" does not borrow-check as
written. NET-04.2e1 replaces it with a compiling form that keeps the semantics:
one closure receives the exclusive shard borrow and derives any view inside it,
and view lifetimes are bound to the shard borrow, not to a handle. The e1 agent
reports the concrete signature to Root before implementing it and proves it with
compile and `compile_fail` tests. No new authority or handle storage in App.

## R3-A2 — lane throughput (NET-04.2e1, NET-04.2f)

Movement is coalesced per session in the session lane. While the newest pending
item of a session is a movement/rotation input that has not been admitted to the
simulation, a newer movement input replaces it (latest position, rotation and
on-ground). Movement is never merged across a teleport confirmation or resync,
and non-movement inputs are never coalesced or reordered: a movement that
arrives after a non-movement input is queued behind it. Live activation also
declares a per-shard player limit; the value comes from the DUR-05b measurement
and is not claimed here.

## R3-A3 — stop deadline (DUR-05a)

The stop sequence gets a configurable total deadline. When it expires, shutdown
ends with `Unresolved { StopDeadline }`, affected UUIDs enter L1 quarantine and
the last confirmed durable state stays authoritative. Phase B runs recovery
boundaries back-to-back, bounded by the deadline, instead of on the 50 ms game
tick. The default deadline must be shorter than common panel stop timeouts and
is documented with the operator note that the panel timeout must exceed it. The
formula `12 000 + 8·N_B` remains only an upper safety bound.

## R3-A4 — crash window (DUR-05b, acceptance)

A hard kill in any stop phase leaves the durable state consistent through
atomic CAS transactions, but process-kill and power-loss acceptance remain open
(DUR-03.3 did not accept them). Before live activation, a periodic player
autosave with a configurable interval is required, and the maximum loss window
for pose, mode and selection is declared to operators.

## R3-A5 — quarantined entity pending (planned SIM ticket, activation gate)

A quarantined entity pending keeps advancing its projection as DUR-04.2
specifies, which drives `CounterRecoveryRequired` and leaves the whole shard
`Degraded`. This must be resolved in a separate simulation ticket before live
activation, so that one quarantined item cannot degrade the shard permanently.
Until then `Degraded` stays typed and operator-visible, and live activation is
gated on that ticket.

## R3-A6 — failing recovery boundary (DUR-05a)

`GenerationExhausted` in a recovery boundary is a terminal typed shard fault:
the shard stops admitting inputs, shutdown ends `Unresolved` and affected UUIDs
enter L1 quarantine. A `confirmed item bytes` preflight refusal follows the
`AllocationFailed` path and counts toward `F_max`, ending in `Degraded`. Neither
path applies partial state.

## Hints

R3-H2: SIM-03.4b may add a read-only diagnostic accessor for the input that a
refusal belongs to; correctness must not depend on it. The remaining hints
R3-H1 and R3-H3–H7 are carried into the runtime tickets named in the round-3
review.

## Runtime authorization

NET-04.2e1 (`ferrumc-session` only) is authorized now; its only prerequisite is
the accepted NET-04.2c contract. SIM-03.4b, NET-04.2e2, DUR-05a, NET-04.2f and
DUR-05b follow the contract's ticket order and are authorized separately.

## Amendment V1 to §7.3 — disconnect retry counting (12.10.2026)

Found by the independent NET-04.2e2 runtime review
([round 1](NET-04_2e2_NATIVE_SESSION_LIFECYCLE_REVIEW.md), V1): counting
disconnect retries per applied normal boundary (`applied > sent_at`) lets a lane
backlog caused by other players quarantine an uninvolved player as
`DisconnectNotApplied`. From now on a retry counts only after the previously
sent disconnect input was actually taken out of the lane and a normal boundary
was applied afterwards while the player stays connected; a backlog never counts.
"Taken out" is measured at the lane receiver: a disconnect that the driver then
holds because the simulation enqueue refused it may count at most one retry
early ([round 2](NET-04_2e2_NATIVE_SESSION_LIFECYCLE_REVIEW_ROUND2.md), R2-H1).
The limit stays one send plus eight retries before
`Quarantined{DisconnectNotApplied}`. Implemented in NET-04.2e2 and marked there
as "§7.3-Nachtrag V1 (Root)".

## Conditions carried to NET-04.2f (12.10.2026)

- Receiver replacement follows §4.3 exactly: replace first, then drain the old
  receiver completely before the new one, proven by a test. A driver that drops
  an undrained old receiver leaves the record in `AwaitDetach`, skips its final
  save, keeps the simulation player connected and makes stop phase A end
  `Unresolved` (NET-04.2e2 review round 2, R2-H2 and the f condition).
- The native path must not route game-mode events through
  `mirror_game_mode_change` (the connection inventory authority) while keeping
  the teleport stamping of `mirror_outbound`.
- Chunk tickets the driver acquires for a player are released by the driver
  (SIM-03.4b review, H3).
- The held-slot resync needs a simulation producer for the selected slot first
  (planned simulation ticket; NET-04.2e2 delivery, A3 partially met).
