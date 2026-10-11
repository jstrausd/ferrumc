# GEN-05.9 — surface rules runtime authorization

Root authorization, 11.10.2026. The concrete contract and genuine original
oracle are independently CLEAN_CONTRACT after three review rounds:
[contract](GEN-05_9_SURFACE_RULES.md) (SHA-256 `1906f57e`, freeze
`.codex-tmp/gen05_9/round3/delivery/freeze-manifest.json` SHA-256 `9f09cbd5`),
[review 1](GEN-05_9_SURFACE_RULES_REVIEW.md) (fabricated flat-probe context),
[review 2](GEN-05_9_SURFACE_RULES_REVIEW_ROUND2.md) (replay rule against P4,
budget behaviour), [review 3](GEN-05_9_SURFACE_RULES_REVIEW_ROUND3.md)
(CLEAN_CONTRACT). The original [work order](GEN-05_9_SURFACE_RULES_TASK.md)
stays binding.

Runtime owns only `server/crates/ferrumc-world/**`, new evidence under
`.codex-tmp/gen05_9-runtime/**` and a new
`GEN-05_9_SURFACE_RULES_RUNTIME_DELIVERY.md`. The contract, its reviews, the
frozen oracle evidence and historical GEN-05.x artefacts are read-only. No new
dependencies, lockfile or generated changes, no other crate, no Git and no
further agents. Root owns the board, integration and consumers.

## Root decisions for the runtime

- **Contract conditions A1–A8 (§13)** are binding as written: replay classes
  `full`, `partial` and `refused` exactly as `replay.tsv` states; the Paper tree
  form from the fixture; the work budget `467 + m` with the stated refusal order
  and continuation rule; measured retained bytes with N-1/N/N+1; the public
  `pub mod surface_rules` in `lib.rs`; the fixture under
  `server/crates/ferrumc-world/tests/fixtures/surface-rules-original-1.21.8/`
  and the reference tooling under
  `server/crates/ferrumc-world/tests/reference/surface-rules/`; INVARIANTS and
  README sections; distinguishable policy refusals and the steep-timing control
  test.
- **A5, `clay_bands_offset`:** the direct bit comparison is mandatory. The
  frozen oracle holds no genuine clay values, so this ticket may run an extended
  copy of the original helper that additionally records the genuine clay
  values. The copy and its outputs live only under `.codex-tmp/gen05_9-runtime/`
  with pinned inputs and the same normal and Python `-O` guard discipline; the
  frozen oracle under `.codex-tmp/gen05_9/` is never modified. Per review hint
  R3-H2, the delivery states the exact column set and the pass criterion
  mechanically. Only the Paper basis is accepted.
- **Review hints R3-H1–H3:** `Poisoned` applies to every fallible session method
  (`work_used` stays infallible); source tests also cover
  `MultiNoise(minecraft:nether)`; the level height assumption `(-64, 384)` is
  named where it is checked.
- **Carried GEN-05.8 condition R-A1:** add the test that pins the two-phase halo
  refusal order of `stored_biome`. Production behaviour of `stored_biome`
  stays unchanged.
- **Policies** as accepted in the contract: Paper numerics only with a typed
  refusal for any other profile; `PaperFlatBedrock::Enabled` refused typed;
  resolver mismatches refused in the order Seed, Settings, Source, Geometry,
  ZoomSemantics.

## Baseline and checks

The read-only baseline is the Git archive of the `main` commit that integrates
this authorization, in `.codex-tmp/gen05_9-runtime-base/archive` with
`base.freeze.json`. Work on a private copy with a whole World overlay; never
edit the archive. Report the exact candidate and manifest to Root before the
final checks. Then run whole-World Debug and Release tests, strict all-target
workspace Clippy with `-D warnings`, `fmt --check`, World `rustdoc` with
warnings as errors, the workspace build and the tests of every World consumer,
each with `--jobs 2 --locked` and a fresh target directory. Source bytes stay
unchanged during the checks; freeze and delivery follow only after all are
green.

Out of scope: the SurfaceSystem column driver and writing surface blocks into
chunks (next World ticket), carvers, features, structures, lighting,
natural-world activation, `PaperFlatBedrock::Enabled`, Survival and Java plugins.
No TPS, RSS or full-server performance claims.
