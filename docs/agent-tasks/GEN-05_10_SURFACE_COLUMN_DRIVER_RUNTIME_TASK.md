# GEN-05.10 — surface column driver runtime authorization

Root authorization, 12.10.2026. The concrete contract and genuine original
oracle are independently CLEAN_CONTRACT in the first review round:
[contract](GEN-05_10_SURFACE_COLUMN_DRIVER.md) (SHA-256 `bd69a4ea`, freeze
`.codex-tmp/gen05_10/delivery/freeze-manifest.json` SHA-256 `ce970306`),
[review](GEN-05_10_SURFACE_COLUMN_DRIVER_REVIEW.md) (SHA-256 `c5cf5ab5`). The
reviewer reproduced the oracle in normal and Python `-O` mode byte-equal,
confirmed that the recording proxy changes nothing (72/72 groups equal with and
without it), rebuilt the column model independently and traced the three extra
Paper `tryApply` calls to the NOISE step. The original
[work order](GEN-05_10_SURFACE_COLUMN_DRIVER_TASK.md) stays binding.

Runtime owns only `server/crates/ferrumc-world/**`, new evidence under
`.codex-tmp/gen05_10-runtime/**` and a new
`GEN-05_10_SURFACE_COLUMN_DRIVER_RUNTIME_DELIVERY.md`. The contract, its review,
the frozen oracle evidence and historical GEN-05.x artefacts are read-only. No
new dependencies, lockfile or generated changes, no other crate, no Git and no
further agents.

## Root decisions

- **Policies D1–D5 accepted** as recommended: D1 closed state alphabet, with
  condition A5; D2 work admission once per chunk; D3 all-or-nothing publication
  into a reusable buffer; D4 centre domain `[−1874999, 1874998]` enforced through
  the producers, with condition A1; D5 `OCEAN_FLOOR_WG` and the post-processing
  order are not published, fluid flags are published as a set. All are FerrumC
  policy, not original behaviour.
- The driver lives in the submodule `surface_rules::column`; no new public
  module. One fresh GEN-05.9 session per chunk with the contract budget.

## Conditions from the review (binding)

- **A1:** `refused:CentreOutsideDomain` gets a pinned typed refusal: the halo
  chunk's `ChunkOutOfRange`, the resolver's `CoordinateDomain`, no call into
  `build_chunk`, and tests at the admitted edges.
- **A2:** the preliminary-surface corners are bound to their producer, or
  documented as caller-trusted input and tested as such.
- **A3:** control test C3 uses the real write path, covers changes in an earlier
  and in the current column, and a mutant that passes post-NOISE heights fails.
- **A4:** control test C5 runs through the internal `cfg(test)` routines and
  iterates the frozen upper bound from `min(start, 320)`.
- **A5:** the D1 alphabet is derived at bind time and checked to be 35 for all
  three settings, not hand-coded.
- **A6:** the runtime checks the fixture file digests before use and classifies
  states by name, so a flipped `blocks_motion` flag cannot pass.
- **Carried from GEN-05.9:** the combined wrong-seed and wrong-settings test
  (RR-H1) and the biome-memo laziness test through `work_used` (RR-H2).

Every expectation is a mechanical projection of genuine original outcomes;
policy and malformed-input tests stay separate.

## Baseline and checks

The read-only baseline is the Git archive of the `main` commit that integrates
this authorization, in `.codex-tmp/gen05_10-runtime-base/archive` with
`base.freeze.json`. Work on a private copy with a whole World overlay. Report the
exact candidate and manifest to Root before the final checks. Then run
whole-World Debug and Release tests, strict all-target workspace Clippy with
`-D warnings`, `fmt --check`, World `rustdoc` with warnings as errors, the
workspace build and the tests of every World consumer, each with `--jobs 2
--locked` and a fresh target directory. Source bytes stay unchanged during the
checks; freeze and delivery follow only after all are green.

Out of scope: carvers, features, structures, lighting, the registry
`blocksMotion` and fluid tables (needed later by carvers and features),
natural-world persistence or activation, Nether and End, Survival and Java
plugins. No TPS, RSS or full-server performance claims.
