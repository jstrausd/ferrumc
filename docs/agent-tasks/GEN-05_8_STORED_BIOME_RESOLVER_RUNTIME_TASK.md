# GEN-05.8 — stored biome resolver runtime authorization

Root authorization, 11.10.2026. The concrete contract and genuine original
oracle are independently CLEAN_CONTRACT:
[contract](GEN-05_8_STORED_BIOME_RESOLVER.md) (SHA-256 `75ffda46`),
[review](GEN-05_8_STORED_BIOME_RESOLVER_REVIEW.md) (SHA-256 `c85ff297`).
The reviewer reproduced the whole oracle in normal and Python `-O` mode:
all 28 differential files and both manifests are byte-equal, all 15 guards
refuse, and all 211 frozen evidence hashes still match
(`.codex-tmp/gen05_8/delivery/freeze-manifest.json`, SHA-256 `d838de6c`).

The original [work order](GEN-05_8_STORED_BIOME_RESOLVER_TASK.md) remains
binding. Runtime owns only `server/crates/ferrumc-world/**`, new runtime
evidence under `.codex-tmp/gen05_8-runtime/**` and a new
`GEN-05_8_STORED_BIOME_RESOLVER_RUNTIME_DELIVERY.md`. The contract, its review,
the frozen oracle evidence and historical GEN-05.5/.6/.7 artefacts are
read-only. No new dependencies, lockfile or generated changes, no other crate,
no Git and no further agents. Root owns the board, integration and consumers.

## Root decisions on the review conditions

- **A1 — domain predicate.** Reuse the accepted `admit_chunk`
  (`density/chunk.rs`) by raising it to `pub(crate)`; do not duplicate the
  30,000,000-block arithmetic. Register the new resolver module in `lib.rs`.
  `admit_chunk` itself is not exported publicly. Add boundary tests at the
  first/last admitted chunk and both neighbours on X and Z.
- **A2 — compact fixtures.** Do not check in the 87 MB query JSONL. Derive a
  compact fixture mechanically from the frozen oracle outputs with a committed
  generator, a fixed selection rule that covers every group, seam, height end,
  geometry, seed type and mode, and a validator that re-checks every fixture row
  against the frozen source rows by hash. The 5,568 controlled empty-hook rows
  become the policy case `OutsideNeighborhood`, never original equality.
  The 32 exception rows stay evidence only. No expectation is computed from the
  Rust implementation or from an independent formula.
- **A3 — zoom semantics.** Add a discriminating test that forwards the five
  GEN-05.6 Vanilla/Paper divergence positions through the same composition and
  shows each mode selects its own accepted result. A hard-wired single mode must
  fail this test.
- **A4 — block-domain refusal.** The block X/Z policy refusal gets a dedicated
  typed error kind carrying the axis and the offending coordinate, documented as
  FerrumC policy, not original behaviour. Refusal order is part of the tests.
- **A5 — history provenance.** Runtime evidence and delivery state that the
  Vanilla histories were produced with `-Dmax.bg.threads=1` on one worker.

The open fallback producer for missing chunks does not block: original
`WorldGenRegion` throws in the SURFACE step instead of calling the hook, so the
typed refusal is the faithful counterpart. A genuine fallback producer remains
a separate ticket.

## Baseline and checks

The read-only baseline is the Git archive of the `main` commit that integrates
this authorization, in `.codex-tmp/gen05_8-runtime-base/archive` with
`base.freeze.json` and `baseline.json`. Work on a private copy with a whole
World overlay; never edit the archive. Report the exact final candidate and
manifest to Root before the final checks. Then run whole-World Debug and Release
tests, strict all-target Clippy with `-D warnings`, workspace `fmt --check` and
`rustdoc` with warnings as errors, each with `--jobs 2 --locked`. Source bytes
stay unchanged during the checks; source/evidence freeze and delivery follow
only after all are green. Root independently reviews and integrates.

Out of scope: surface-rule compilation and the SurfaceSystem driver (next World
tickets), carvers, features, structures, lighting, natural-world persistence or
activation, Survival and Java plugins. No TPS, RSS or full-server performance
claims.
