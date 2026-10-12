# SIM-03.4c — selected slot in the connected player view, runtime authorization

Root authorization, 12.10.2026. NET-04.2e2 could only meet condition A3 of its
authorization partially: corrections and resync use the current simulation pose
and mode, but the held-slot resync has no simulation producer.
`ConnectedOwnedPlayerView` exposes player, generation, position, look, game mode
and hashed seed, and the shard offers `owned_player_slots` and
`owned_player_carried`, but nothing returns the currently selected hotbar slot of
a live native player ([NET-04.2e2 delivery](NET-04_2e2_NATIVE_SESSION_LIFECYCLE_DELIVERY.md),
[review round 2](NET-04_2e2_NATIVE_SESSION_LIFECYCLE_REVIEW_ROUND2.md)). The
held-slot resync is a precondition for live activation
([Root decisions](NET-04_2d_NATIVE_DRIVER_ENDPOINT_ROOT_DECISIONS.md), R3-A2/A3).

Runtime owns only `server/crates/ferrumc-sim/**`, new evidence under
`.codex-tmp/sim03_4c-runtime/**` and a new
`SIM-03_4c_SELECTED_SLOT_VIEW_DELIVERY.md`. No other crate, no new dependencies,
no lockfile or generated changes, no Git and no further agents.

## Scope

- A read-only accessor on `ConnectedOwnedPlayerView` that returns the live
  selected hotbar slot of exactly the viewed admission, as a typed slot value,
  without allocation. It reads the same state that the accepted `select_hotbar`
  input mutates and that the native snapshot persists; no second source of
  truth. Document where the value comes from and how it relates to
  `NativePlayerSnapshot::selected`.
- Tests on Memory and redb ownership admission in counter mode with genuine
  work and completion: the value after bootstrap and restore, after an applied
  `select_hotbar`, after a rejected or not yet applied one, after rejoin, and
  that a view of a detached, stale or foreign handle cannot be obtained.
- The low documentation finding N1 of the SIM-03.4b review: correct the number
  of pending-map passes in the `release.rs` comments and in `INVARIANTS.md` to
  three, with the bound of 833 visits unchanged. Comments and documentation
  only; no behaviour change in `release.rs`. Root corrects the historical
  SIM-03.4b delivery note at integration.

The session-side resync wiring and the driver remain with NET-04.2f.

## Baseline and checks

The read-only baseline is the Git archive of the `main` commit that integrates
this authorization, in `.codex-tmp/sim03_4c-root-base/archive` with
`base.freeze.json`. Work on a private copy with a whole Sim overlay. Report the
exact candidate and manifest to Root before the final checks. Then run Sim Debug
and Release tests, strict all-target workspace Clippy with `-D warnings`,
`fmt --check`, Sim `rustdoc` with warnings as errors, the workspace build and the
tests of every Sim consumer (`session`, `app`, `testkit`, `bench`), each with
`--jobs 2 --locked` and a fresh target directory. Source bytes stay unchanged
during the checks; freeze and delivery follow only after all are green.

Out of scope: isolation of quarantined pendings (R3-A5, separate ticket), live
activation, Survival and performance claims.
