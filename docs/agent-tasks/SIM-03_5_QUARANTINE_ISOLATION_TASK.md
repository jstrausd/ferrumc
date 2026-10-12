# SIM-03.5 — isolate quarantined item pendings from the shard, work order

Root work order, 12.10.2026. Decision R3-A5 of the
[NET-04.2d Root decisions](NET-04_2d_NATIVE_DRIVER_ENDPOINT_ROOT_DECISIONS.md)
requires that one quarantined item pending cannot degrade a whole shard
permanently, before live activation. The mechanism is documented in the
[NET-04.2d review round 3](NET-04_2d_NATIVE_DRIVER_ENDPOINT_REVIEW_ROUND3.md)
(R3-A5): a `Refused(CasConflict)` in counter mode sets the pending to
`Quarantined` and keeps `pending.counters`; the plan binds the projection of
every pending with counters without a status filter and raises `claims` per
resident normal boundary, so after at most `limits.defer` normal boundaries the
shard enters `CounterRecoveryRequired` for good. The NET-04.2e2 runtime also
showed that leave-save quarantine then becomes unreachable. DUR-04.2
([brief](DUR-04_2_OWNED_ITEM_COUNTERS.md), lines 604–606) prescribes that the
private counter projection keeps advancing for a quarantined pending, so the
fix changes an accepted contract and starts with a concrete contract.

Preparation owns a new `SIM-03_5_QUARANTINE_ISOLATION.md` and
`.codex-tmp/sim03_5/**` only; production stays read-only. A later runtime
authorization owns only `server/crates/ferrumc-sim/**`. Root owns Git, the board,
integration and any amendment to DUR-04.2. No new dependencies, other agents or
edits to historical artefacts.

The concrete contract must settle:

- Why DUR-04.2 keeps advancing the projection of a quarantined pending, which
  durability or counter invariant that protects, and whether the original
  Paper/Vanilla item-age and despawn semantics require it. Read the accepted
  DUR-04.1 and DUR-04.2 contracts, reviews and runtime and the genuine original
  item entity sources.
- The smallest change that keeps every protected invariant while one
  quarantined pending can no longer drive the whole shard into
  `CounterRecoveryRequired`: for example freezing the projection of exactly that
  item, excluding it from `claims`, or a typed per-item quarantine state with a
  bounded resolution path. Name the exact functions and fields, the effect on
  stop readiness (`owned_stop_ready`), on release (SIM-03.4b), on leave-save
  quarantine (NET-04.2e2) and on recovery mode (NET-04.2d §6.1a).
- How the quarantined item becomes durable or is resolved later (operator
  action, reconcile, restart) without inventing state, and what the player and
  the operator see.
- A complete test matrix on Memory and redb ownership admission in counter mode
  with genuine work and completion, including the R3-A5 reproduction from the
  NET-04.2d review and the NET-04.2e2 delivery, N-1/N/N+1 at `limits.defer`,
  multiple concurrent quarantines, rejoin, release and stop.
- The exact amendment text to DUR-04.2 that Root would adopt.

Root independently reviews the contract before authorizing production. No
activation, Survival or performance claims.
