# DUR-03.3b — independent startup receipt reuse runtime review

Root assignment, 12 October 2026. Review the frozen whole Storage candidate
independently before integration. This is a new review round, not an extension
of an earlier completed reviewer. World generation remains the operator's
priority. No Storage, Sim or App activation is authorized by this assignment.

## Scope and immutable basis

Read AGENTS.md, CLAUDE.md, candidate Storage INVARIANTS, architecture/storage,
ADR-0004/0007/0009 and PAPER_REFERENCE. Read the complete original
[DUR-03.3 contract](DUR-03_3_PLAYER_BLOCK_DURABILITY.md), its original source
review/design evidence and contract review, runtime reviews 1, 2 and 3, their
responses in the original delivery, the complete
[DUR-03.3b assignment](DUR-03_3b_STARTUP_RECEIPT_REUSE_TASK.md) and new
[delivery](DUR-03_3b_STARTUP_RECEIPT_REUSE_DELIVERY.md). Inspect the actual redb
2.6.3 guard/read implementation and affected complete callers/callees.

Write only the new
`docs/agent-tasks/DUR-03_3b_STARTUP_RECEIPT_REUSE_RUNTIME_REVIEW.md` and
`.codex-tmp/dur03_3b-independent-review-r1/**`. No Git, agents, source/doc fixes,
dependency changes, or edits to historical fixtures, freezes, logs and reviews.
Storage and concurrent World WIP are readonly; do not build against either.

Root supplies `.codex-tmp/dur03_3b-review-root-base/`:

- `archive/`: genuine accepted main
  `4225c21cfcb648a075f807b32e586276fe6661c6`, 1,448 files;
  `base.freeze.json` SHA256
  `b446c4ac2f811ab8236f86df0867fe0d31f3850da4f9ea995463c4d4e448a765`.
- `combined/`: that archive plus the whole frozen 93-file Storage candidate
  and new delivery, 1,481 files, including 324 accepted World files;
  `combined.freeze.json` SHA256
  `023f95d55a5ce62c75f5e7c786d4990e5ac6635b62a990cb820ca1d0e590d002`.
- `basis-comparison.json`: the author's 1,470-file combined source and Root's
  main differ outside Storage in exactly 11 documentation paths. Every byte
  outside docs/Storage is identical; verify this rather than infer from commits.

Use a private writable copy of this immutable combined tree and a separate
private target. Never overlay mutable shared World or other unaccepted features.
Own probes/test products belong only in the private review directory. Builds use
`/home/jstrauss/.cargo/bin/cargo`, jobs 2, locked. Keep original compiler logs and
test sources; failures remain evidence and must be explained.

Candidate author area:
`.codex-tmp/dur03_3b-runtime/implementation-r1/`.

| Pin | SHA256 |
| --- | --- |
| Original contract | `2edfc588fd8d7e96753f6beb4bb7c2ad50a064e4c266681ab5ff94550c4342ab` |
| Original design freeze | `3710c57265084f9a6cd59d48fc77b9340fcd255bd0f235545c7ca639b92de92f` |
| Original 1,224-file archive manifest | `c148936b29cc58d7ac25b2904618523c3b3a163476a2bf380b1d00fadf07c7b0` |
| Round-6 candidate manifest | `d21db90ec56958c965c2a8fb288d21b82ad5af22c8bda9cd0d471cd894abd0d7` |
| Round-6 evidence freeze | `0335589a6f659d79a0750e53487d960356ba567138e6829f33ad8dc8f9b73e0c` |
| Round-3 evidence freeze | `88ab8eafd921d06a5f2de215a4fe010dd800a18e3e22cf89490c4638633406fd` |
| New 93-file candidate manifest | `52bcdfcaa7fdc9d4424370a0d282f9f266df8b5b626374647c74372a767be79c` |
| New candidate content digest | `068fc2281212ba93e4ddcebf46f22101d034a8b5843fe7777da27d43372f8dcd` |
| Author combined freeze (1,470 files) | `02b06c4622c41daa65a0524ca9054974bbbdd2b07be9b2ac203069f15a767290` |
| New evidence freeze (42 artifacts) | `938ef98bd38159dd8cf601325393baa192e4f5bc403ac820e266cf045d36b04e` |
| New delivery | `d24b3ad6939e86f62f31b230ecd2357ec763e73b3f9b15cca284bc50486cea38` |

The candidate manifest uses repo-relative `entries`. `candidate-src-r1/` is
crate-root layout: strip `server/crates/ferrumc-storage/` when checking its files.
`combined-candidate-r1/` uses full repo-relative layout. The evidence manifest
uses area-relative `evidence` plus a separately pinned delivery. Root checked
all three candidate locations, all combined source and all evidence bytes in
`.codex-tmp/orchestration/dur03_3b-runtime-intake.json`; repeat independently.
The author's genuine main was `55f362f6`, baseline manifest `15bd5568…b33`;
the old erroneous `35e2556e…` reported a `.dockerignore` hash. The corrected
baseline is binding; the first erroneous intake remains historical evidence.

## Prior findings, responses and unresolved objections

Review 1 reported A1 whole-receipt endpoint amplification in runtime reads and
A2 failed Fresh-open blocking retries; H1 v1 refusal order, H3 whole loser-state
assertions, H4 independent bounds, H7 undocumented lint allowance, and disclosed
startup work/legacy/MAX/commit-fault limits. Review 2 verified A1/A2/H1/H3/H7 fixes
and partial H4 improvement, then reported B1 flat 16-KiB entity encoding charges,
R2-H1 duplicate uncharged Q parse, R2-H2 cleanup masking a primary error, and
R2-H3 the retained inode-check/path-unlink window. Review 3 verified the round-6
B1 actual encoding-length correction, shared frame reuse in runtime reads and
primary-error preservation; R2-H3 remains accurately disclosed and unchanged.
Earlier accepted authority/CAS/replay/atomicity and Format6 observations remain
binding and require regression assessment. Do not erase historical findings.

Review 3 remained FINDINGS because **R6-B2** reread/hashed/parsed an entire Q for
every scoped Journal row on startup. Its real public probe 8 committed a valid
three-column Q=2,425,362 with 1024 effects, then failed reopen with its original
1-GiB relation allowance. The branch alone cost 4,967,258,112 bytes. A second Q
parse there was uncharged. Root ordered a fix before integration, without any
limit increase or temporary waiver; R6-B2 is the remaining targeted gate.

The new delta has six changed files (INVARIANTS, README, block_validation.rs and
tests, test-only engine/raw instrumentation) and two new integration tests.
Production now consumes each contiguous operation range under one ordinary
borrowed redb receipt guard and one charged frame_q result, dropping both before
the next operation. Every row must retain scope/operation/ordinal/sequence/effect
agreement. Complete endpoint/reverse/counter/catalog/index proofs remain.
Writer/apply, capacities and persisted grammar reportedly remain byte-identical.
Check the entire preserved contract and actual delta, not only the summary.

Author reports Storage Debug/Release each 360 passed, no ignored; strict workspace
all-target Clippy/fmt/build and Storage rustdoc green; consumers 884 passed,
one Java test ignored. These remain author's runs. Root proved the current-main
dependency bytes identical, but this does not make them reviewer-executed checks.
Avoid repeating full green suites solely to produce another identical matrix;
audit those exact logs/manifests and run meaningful fresh independent probes.
Broaden checks if new defects, source drift or unresolved concerns require it.

## Independent acceptance required

1. Verify whole base/candidate/evidence/history/input manifests before and after,
   including original 461 dependency, 45 redb and 82 Paper pins. Read all changed
   files completely and affected old proof/reader/budget paths. Review safe guard
   lifetimes, actual page-backed ownership, one-receipt retention, all debit sites,
   checked arithmetic, error precedence and scratch cleanup preservation.
2. Establish the **real original 1-GiB probe-8 reopen** with exact original fixture,
   receipt, old-incarnation replay, raw pairs/views, all 17 logical tables and empty
   owned scratch. The unchanged old probe intentionally asserts refusal and now
   exits 101; retain that diagnostic as such. The separate adapted test may change
   only the stale expected result/test name. Neither raising limits nor skipping
   the actual close/reopen resolves R6-B2.
3. Independently derive full cumulative scan work, not just Journal receipt work.
   Check genuine N-1/N/N+1 for small-effect/large-Q, multiple operations including
   zero-effect receipts and different scopes, dense near32-MiB entity effects.
   Author N values are 42,087,955; 61,761,009; 468,808,126. Audit the actual model,
   immutable admitted shapes and full table/relation/endpoint/index charges;
   candidate-reported requirements are not an oracle. Execute fresh own boundary
   coverage sufficient to challenge the derivation, documenting any cases checked
   by source/log audit rather than independently executed.
4. Challenge corruption across operation transitions: scope/ID/ordinal/sequence/
   effect/range, interleaving, old-operation reuse, absent/missing/extra rows and
   zero-effect receipts. Both directions and the full endpoint proof must refuse
   coherent receipt+Journal forgery with a correct digest; a relation-only pass
   is deliberately insufficient. Refusals preserve all 17 logical tables.
5. Review actual test-only instrumentation (2 reads/parses/hashes for two operations,
   peak1 guard, zero guards afterward), its coverage and production removal. Check
   that no Q-copy/map/reset/discount/unsafe/global/cap change conceals work. Probe
   genuine once-per-operation behavior and error drops independently as needed.
6. Assess regressions to all prior authority/CAS/replay/no-op/history/atomicity/
   union/payload/endpoint/counter/bypass/uncertainty behavior. Keep arbitrary
   retained-history costs, runtime hard-byte residuals, scratch unlink window and
   separate DUR-03.3a migration explicit. No App activation or gameplay fallback.

Publish severity, exact symbols, independent reproduction, contract impact and
proposed remedy for every finding. Verdict CLEAN_RUNTIME_REVIEW requires R6-B2
closed and no blocking contract/runtime objection; otherwise FINDINGS. Distinguish
source/log audit, author checks and fresh reviewer execution. Freeze new evidence
and independently verify unchanged source/outside bytes at completion. Only Root
integrates. No full Survival, Paper/client/plugin, performance/RSS, migration,
process-kill/power-loss or App/Sim activation claim.
