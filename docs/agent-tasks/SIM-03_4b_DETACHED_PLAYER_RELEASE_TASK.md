# SIM-03.4b — detached native player release, runtime authorization

Root authorization, 11.10.2026. This simulation ticket comes from the accepted
NET-04.2d contract
([NET-04_2d_NATIVE_DRIVER_ENDPOINT.md](NET-04_2d_NATIVE_DRIVER_ENDPOINT.md),
SHA-256 `e0530e0a`) and its
[Root decisions](NET-04_2d_NATIVE_DRIVER_ENDPOINT_ROOT_DECISIONS.md). Its scope
is fixed by the contract: §7.5, the SIM-03.4b row of §13, the SIM-03.4b rows of
the §14 matrix and the stop readiness part of §10.4. The prerequisites
SIM-03.4a and DUR-04.2a are integrated.

Runtime owns only `server/crates/ferrumc-sim/**`, new evidence under
`.codex-tmp/sim03_4b-runtime/**` and a new
`SIM-03_4b_DETACHED_PLAYER_RELEASE_DELIVERY.md`. No other crate, no new
dependencies, no lockfile or generated changes, no Git and no further agents.
Root owns integration and the independent review.

## Scope

- `saved_after_detach`, set by the simulation itself when it applies the genuine
  autosave completion of exactly the detached admission; the latest confirmed
  save wins, and any later pending or uncertain outcome resets it or blocks
  release (G5, R3-2).
- Read-only `detached_owned_player_status(&self, original)` returning the
  getter type of §7.5, without allocation; stale, foreign, snapshot or connected
  handles are refused typed.
- `release_detached_owned_player(&mut self, original)` without a caller query,
  returning a `ReleasedOwnedPlayer` that carries no authority. It removes the
  player state, its bytes and every counter consistently, and refuses typed
  without partial mutation in every A7 case of §14: connected, lifecycle cell
  not acknowledged, lifecycle not initialised, no `saved_after_detach`, pending,
  bootstrap, restore or queued input of the same player, publication pin, item
  or chunk reservation, collector binding, snapshot, foreign or stale handle,
  repeated release. No automatic eviction; bounded work per call as the contract
  states.
- Admission bit `lifecycle_acked` set by `acknowledge_collector_lifecycle`
  (D8 of §7).
- Read-only `owned_stop_ready()` sharing one internal predicate helper with
  `ownership_quiescent`, without allocation; it is true exactly when a stop is
  accepted as the only input of the next normal boundary (§10.4, R3-1).
- Re-export of `OwnershipTransferId` and `OwnershipReceiptQuery` from
  `ferrumc-sim`, so `ferrumc-session` never depends on storage (R2-A4).
- Optional per Root decision R3-H2: a read-only diagnostic accessor for the input
  a refusal belongs to; correctness must not depend on it.

Bound conditions: A7, G5, R3-1, R3-2, R2-A4, R2-H3, R2-H5, R2-H6 of the contract.
Every SIM-03.4b matrix row is a real test on Memory and redb ownership admission
in counter mode with genuine work and completion, no fake facts. Refusal,
N-1/N/N+1 and race tests stay separate from success cases.

## Baseline and checks

The read-only baseline is the Git archive of the `main` commit that integrates
this authorization, in `.codex-tmp/sim03_4b-root-base/archive` with
`base.freeze.json`. Work on a private copy with a whole Sim overlay; never edit
the archive. Report the exact candidate and manifest to Root before the final
checks. Then run Sim Debug and Release tests, strict all-target workspace Clippy
with `-D warnings`, `fmt --check`, Sim `rustdoc` with warnings as errors, the
workspace build and the tests of every Sim consumer, each with `--jobs 2
--locked` and a fresh target directory. Source bytes stay unchanged during the
checks; freeze and delivery follow only after all are green.

Out of scope: session retention and release callers (NET-04.2e2), the App driver
and stop deadline (DUR-05a), the quarantined-pending isolation of R3-A5 (its own
ticket), live activation, Survival and performance claims.
