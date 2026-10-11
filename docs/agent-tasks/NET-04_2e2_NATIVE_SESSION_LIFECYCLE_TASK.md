# NET-04.2e2 — native session lifecycle, runtime authorization

Root authorization, 11.10.2026. Second session ticket of the accepted NET-04.2d
contract ([NET-04_2d_NATIVE_DRIVER_ENDPOINT.md](NET-04_2d_NATIVE_DRIVER_ENDPOINT.md),
SHA-256 `e0530e0a`) and its
[Root decisions](NET-04_2d_NATIVE_DRIVER_ENDPOINT_ROOT_DECISIONS.md). Its
prerequisites are integrated: NET-04.2e1
([review](NET-04_2e1_NATIVE_ENDPOINT_SESSION_RUNTIME_REVIEW.md)) and SIM-03.4b
([review](SIM-03_4b_DETACHED_PLAYER_RELEASE_REVIEW.md)).

Runtime owns only `server/crates/ferrumc-session/**`, new evidence under
`.codex-tmp/net04_2e2-runtime/**` and a new
`NET-04_2e2_NATIVE_SESSION_LIFECYCLE_DELIVERY.md`. No other crate, no new
dependencies, no lockfile or generated changes, no Git and no further agents.
Root owns integration and the independent review.

## Scope

The NET-04.2e2 row of contract §13 and every e2 row of the §14 matrix: join
admission with a reservation that carries its table entry, the Active
transition, the retention record holding the original handle until release,
lineage assignment, the leave-save state machine including R2-A1, the
session-side `service/release_native_retention` closure, detached intents,
`adopt/retire_unbound` (".2c amendment G3"), the R1/R2 retirement amendments,
audience and reconcile, pose and resync. Bound contract conditions: A2, A5
(session part), A6, B2, B3, R3-3, R2-A1, R2-A3 (lineage). The single producer
for leave save, release and the G3 disconnect stays in the session; the driver
holds only an opaque token.

## Conditions carried from earlier reviews

From the NET-04.2e1 runtime review:

- A1: the join reservation allocates the table entry that the session or the
  G3 adoption later takes over, so adoption stays capacity-infallible (R3-3,
  §5 step 6). The current two plain map inserts in `native.rs` are replaced.
- A2: a refused revalidation must not hold a disconnect forever; it ends in a
  defined disposition and `retry_native_disconnects()` reaches zero.
- A3: corrections and resync use the current simulation pose, not a pose read
  before older lane items were applied. This must hold before any activation.
- A4: `native_lane_input_live(Control)` returns false after the DetachFact.
- A5: `disconnect_native` returns the retention token.
- A6: the session part of real teleport IDs; the allocation itself may stay with
  NET-04.2f if the contract places it there, which the delivery states.
- A7: the e1 signature deviations (unregister returns the receiver on refusal,
  `session_key()` is optional, renamed native routing functions) are recorded in
  the delivery's decision log.

From the SIM-03.4b review and Root decisions:

- A foreign owner reports `StaleHandle`; the session treats stale and foreign
  handles alike as quarantine, never as success and never as silent loss.
- H1: a real session test proves the session compiles and works against
  `ferrumc_sim::OwnershipTransferId` and `OwnershipReceiptQuery` without a
  storage dependency in production code.
- Release goes only through `release_detached_owned_player(original)` after
  `detached_owned_player_status` shows the confirmed save; `lifecycle_acked` is
  set through `acknowledge_collector_lifecycle`.

Every e2 matrix row is a real test with bounded router channels and genuine
Memory and redb ownership admission in counter mode, with real work and
completion, no fake facts. Refusal, N-1/N/N+1 and race tests stay separate from
success cases. Storage and world remain test-only dependencies of the session.

## Baseline and checks

The read-only baseline is the Git archive of the `main` commit that integrates
this authorization, in `.codex-tmp/net04_2e2-root-base/archive` with
`base.freeze.json`. Work on a private copy with a whole session overlay; never
edit the archive. Report the exact candidate and manifest to Root before the
final checks. Then run session Debug and Release tests, strict all-target
workspace Clippy with `-D warnings`, `fmt --check`, session `rustdoc` with
warnings as errors, the workspace build and the tests of `app` and `testkit`,
each with `--jobs 2 --locked` and a fresh target directory. Source bytes stay
unchanged during the checks; freeze and delivery follow only after all are green.

Out of scope: the App driver, permits, L2 token table, gate and stop sequence
(DUR-05a, NET-04.2f), live activation, Survival and performance claims.
