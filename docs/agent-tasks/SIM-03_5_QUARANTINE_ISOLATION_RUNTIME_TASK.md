# SIM-03.5 — quarantined item pending isolation, runtime authorization

Root authorization, 12.10.2026. The concrete contract V2
[SIM-03_5_QUARANTINE_ISOLATION.md](SIM-03_5_QUARANTINE_ISOLATION.md) is accepted
after two independent review rounds:
[round 1](SIM-03_5_QUARANTINE_ISOLATION_REVIEW.md) (FINDINGS A1–A6, SHA-256
`27a4709c`), [round 2](SIM-03_5_QUARANTINE_ISOLATION_REVIEW_ROUND2.md)
(FINDINGS with only the low text finding N1, SHA-256 `013036c0`). The round-2
reviewer confirmed A1–A6 as fixed, reproduced the full suites on the round-2
prototype (Sim 404, Session 244, Testkit 100, App 375/0/1) and recommended
CLEAN_CONTRACT once N1 is corrected. Root corrected N1 verbatim at adoption (see
the integration note in the brief) and adopted the amendment of §8 verbatim:
DUR-04.2 now carries ten replacements and the new §11, DUR-04.1 two
replacements. The original [work order](SIM-03_5_QUARANTINE_ISOLATION_TASK.md)
stays binding.

Runtime owns only `server/crates/ferrumc-sim/**` (including `INVARIANTS.md`),
new evidence under `.codex-tmp/sim03_5-runtime/**` and a new
`SIM-03_5_QUARANTINE_ISOLATION_RUNTIME_DELIVERY.md`. The contract, both
reviews, the amended DUR-04.1/DUR-04.2 briefs and all historical artefacts are
read-only. No other crate, no new dependencies, no lockfile, Cargo or generated
changes, no public API change, no Git and no further agents.

## Scope

- Implement contract V2 exactly as in brief §5 and §10: counter mode only
  (`item_counters.is_some()`), the DUR-04.1 legacy path stays byte-for-byte
  equal in behaviour. Changes are expected in
  `src/shard/player_ownership/counters.rs`, `src/shard/player_ownership.rs`,
  a new test module `src/shard/player_ownership/counters/tests/quarantine.rs`
  with its `mod` line, and `INVARIANTS.md`.
- `INVARIANTS.md` receives the four blocks of brief §5.4 (`INV-290`, `INV-347`,
  `INV-366`, `INV-386`) verbatim, replacing exactly the old texts named there.
- If the implementation needs any deviation from the contract text, stop and
  return the question to Root as your result; do not decide it yourself.

## Mandatory tests (Memory and redb, counter-mode ownership admission with genuine work and completion)

- Everything in brief §7 "Pflicht für die Runtime-Abnahme": M1–M15 (T1–T13
  including T3b, and T14), R1–R4 as real assertions, EntityDespawn quarantine,
  quarantine in a recovery boundary with save selection in the same boundary,
  SnapshotOnly copy, legacy mode with `recover_player_ownership_completions`,
  second-player pickup with a pre-conflict read, pending load (four maximal
  entity-save quarantines, then `Capacity("pending bytes")`).
- **N1 row:** completion, pickup and save of the same item in the quarantining
  boundary, in both input orders, yield `CasConflict`,
  `Ineligible("quarantined item")`, `Reserved` and zero work; the reservation is
  free after the boundary.
- **Probe A (legacy):** repeated legacy recovery on a quarantined owner; all
  four writers stay `Reserved`, no work against the superseded expectation.
- **Probe B:** outside a fence the refusal order equals the base behaviour,
  as a real test, not a measurement line.
- **Probe D:** pending load exactly at the bound succeeds, one more is refused.
- N-1/N/N+1 at `limits.defer` and multiple concurrent quarantines as in the
  matrix.

Every expectation follows from the contract or from measured base behaviour;
label policy tests as FerrumC policy.

## Baseline and checks

The read-only baseline is the Git archive of the `main` commit that integrates
this authorization, in `.codex-tmp/sim03_5-runtime-base/archive` with
`base.freeze.json`. Work on a private copy with a whole Sim overlay. Report the
exact candidate and manifest to Root in your result before the final checks if
anything is unclear; otherwise proceed. Then run Sim Debug and Release tests,
strict all-target workspace Clippy with `-D warnings`, `fmt --check`, Sim
`rustdoc` with warnings as errors, the workspace build and the tests of every
Sim consumer (`session`, `app` with `--no-fail-fast`, `testkit`, `bench`), each
with `--jobs 2 --locked` and a fresh target directory. Source bytes stay
unchanged during the checks; freeze and delivery follow only after all are
green.

Out of scope: a permanently `Uncertain` pending (O1, own ticket with its own
activation gate), O2 capacity threshold and the e2 leave-save condition
(DUR-05a), in-process resolution (O4), driver backoff (NET-04.2f), live
activation, Survival and performance claims.
