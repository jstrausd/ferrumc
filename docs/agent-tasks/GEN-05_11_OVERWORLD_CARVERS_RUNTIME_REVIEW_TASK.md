# GEN-05.11 — independent cold Carver runtime review, round 1

Root assignment, 12 October 2026. Review the complete frozen candidate044
against the original binding contract and accepted revision2. This is the first
runtime review. Earlier CR11-B1/B2 findings were closed at preparation level;
that closure does not establish implementation correctness. No runtime
acceptance, second World writer or App activation is granted by this assignment.

## Binding brief, findings and responses

Read the complete [runtime assignment](GEN-05_11_OVERWORLD_CARVERS_RUNTIME_TASK.md),
including every acceptance obligation; [original assignment](GEN-05_11_OVERWORLD_CARVERS_TASK.md),
[original contract](GEN-05_11_OVERWORLD_CARVERS.md),
[first contract review](GEN-05_11_OVERWORLD_CARVERS_REVIEW.md),
[correction assignment](GEN-05_11_OVERWORLD_CARVERS_REVISION2_TASK.md),
[revision2 response/contract](GEN-05_11_OVERWORLD_CARVERS_REVISION2.md) and
[clean contract/oracle review](GEN-05_11_OVERWORLD_CARVERS_REVISION2_REVIEW.md).
Original provisions remain binding where explicitly not replaced. Also read
the full [runtime delivery](GEN-05_11_OVERWORLD_CARVERS_RUNTIME_DELIVERY.md),
AGENTS/CLAUDE, PAPER_REFERENCE, archived and candidate World invariants,
relevant architecture/ADRs and accepted GEN-05.3–05.10 producer contracts.
Read full relevant real Java methods/callees and both applied patch layers.

Prior findings and Root's accepted responses:

- CR11-B1: Stored substitution, skipped source calls and clamped direct Y
  cannot establish original lazy biome history. Revision2 requires genuine
  direct climate/selector/zoom, all289 source calls in X-outer/Z-inner order,
  consumed fix-ups interleaved, private derived halo/Y−17, shared lazy memo,
  no session reset. Root accepted this corrected preparation.
- CR11-B2: per-call budgets/resets and empirical maxima cannot establish
  uninterrupted cumulative producer authority. Revision2 supplies structural
  constructor/Noise/Selector/rule bounds, checked private unsigned192 limit/
  used, segment bookkeeping and exact nested errors without changing public
  caps. Root accepted the design; every actual charge/error path and native
  regression remains review-critical.
- RR10-H1–H5: actual y319 production scan/M13; lava-vs-water eroded/frozen
  edges/M14/M15; literal10816/406592 plus new layouts; external allocation
  measurement for Binding/Material/Session/UnclassifiedState; genuinely
  Published→Empty refusal. Author claims these are implemented in candidate044.

Keep all historical findings, original policies and exact responses visible.
The author reports a private finite-f64 cache repair preserving the accepted
400-byte rule session, full wide-fault adapters and cold allocation evidence.
Inspect these changes as production behavior rather than accepting the report.

## Immutable inputs and tested bases

Repository `/home/jstrauss/Schreibtisch/mchosting/ferrumc`, Cargo workspace
`server/`. Root verified all frozen package bytes, exact membership, readonly
permissions, shared/tested World equality and all202 historical fixture/reference
files unchanged. Root receipt:
`.codex-tmp/gen05_11-runtime-root-review-r1/intake.json`.

| Authority | SHA256 / count |
| --- | --- |
| Complete readonly package `.codex-tmp/gen05_11-runtime/delivery/freeze-manifest.json` | `574e54d8dafbd087d238d3d349f83369457664dec157e461ec2179129ce8c162`, 945 members including manifest |
| Source manifest `delivery/source.freeze.json` | `243418db1406675450902b17a4f38c280b48a093faa7f91232e0cc2d1bb4334f`, 362 World files |
| Canonical complete World path→hash digest | `ab77a66d73a92882b661398931a99c6b7190984a1c49d99a13ac01586d735f19` |
| Evidence manifest `delivery/evidence.freeze.json` | `b01410b899a76bce7d63594f9eb1cef436afea1e8c723fb4a7087f4e15c7804a`, 579 files |
| Runtime delivery document | `b251eb841f1f1344d307379c4f64eb295c148dd2cbc18676df59aae3fb2d98e6` |
| Author final summary044 | `65b4dfdfafd849d5c1492b86cef86e798b1f72f1239ef58ea3b4cf923c1fd9b7` |
| Revision2 contract | `ddd6c13188cfffc570ad80df9b9678d534e7c04859f23cfd6ee9e97f003ecbec` |
| Revision2 author freeze,448 files | `46a5d1dc02e913b0a72c0a979b1ddd393f46b52504b13871ab00f21e88f3f267` |
| Revision2 CLEAN review | `2c9a9c7af235ccbaa5bf0ea72e5465b8885d3d34a29707f264130382bb472d0a` |
| Revision2 review freeze,181 files | `2c7a2527b3173f933342b7db70fe0f0c36adcfe6fa72cc2b020c6d32e7da4f26` |
| Revision2 input pins | `d4c9a8c60bf8ad0bac9d6f1d0c3fa8f0f8884c50c52a35caf4a045ed2921651c` |

All `delivery/` paths above are relative to `.codex-tmp/gen05_11-runtime/`.
Use `delivery/source/archive/` and `delivery/evidence/archive/` as immutable
candidate/evidence authorities, plus the original references they pin. Handoff
`.codex-tmp/gen05_11-runtime/handoff-044.json` resolves exact receipts. Verify
full freezes/input authority independently before/after; streaming hashes avoid
loading large artifacts into memory. No historical file may be repaired.

Author complete final checks used genuine accepted main
`47eadc4c14813c201184b552a87ca7b9c6f97db4`,1490 files/324 accepted World/93 Storage,
readonly `.codex-tmp/gen05_11-runtime-root-base-storage47e/archive`;
base.freeze SHA `58ba13d6418133bb49a154682ba1d82533f63a5e6184c6c2a8b07c6c15a47a75`.
All1166 outside-World files remained unchanged. Old4225-base checks remain
supplemental only. Candidate044's own final checks report World Debug/Release
415 passed/7 explicitly driven ignored probes,1550 consumer tests/1 ignored,
workspace alltarget Clippy/fmt/build and World rustdoc green.

Root provides a **fresh genuine current accepted archive**, main
`356670f7714bc42f2626dd68809e6fd1975a5d96`,1499 files:
`.codex-tmp/gen05_11-runtime-root-review-r1/base/archive`;
adjacent `base.freeze.json` SHA
`3788f6be81effe6d4d06db3bf4380319844ed3738dbf80dc90e62a119041d6ae`.
All server runtime/dependency bytes equal the author tested47e base. Eleven
changed paths are docs plus the CI timeout adjustment, fully enumerated by intake.
Expected current archive + whole frozen World + frozen delivery membership:
`.codex-tmp/gen05_11-runtime-root-review-r1/combined.freeze.json`,1538 files,
SHA `628bce18102403a0cef341ea0b5000eb29deaf21cea9b1632fd4fdebc3a085a3`.
Build only a private writable copy in your own area, with separate target;
`/home/jstrauss/.cargo/bin/cargo`, locked/offline, jobs2. No shared WIP checks.

## Independent runtime audit and evidence

Read every changed/new implementation, test, fixture importer/validator and
instrumentation tool completely; inspect complete affected old public and
private producer success/refusal paths. Review delta is38 new/38 modified World
files, no removals, and the delivery document. Pin all sources examined.

1. Verify actual full Surface→Carver production composition: all71 Paper groups,
   S1 first, complete FINAL states/digest, mask/heights/fluid set, starts/RNG/
   source/carver trace,42117 Aquifer state/flag calls,355 lazy fix-ups and actual
   quart/biome chronology. Both refused groups stay typed Empty; Vanilla's25
   discriminating groups retain control labels. Trace equality must not hide
   fixture-fed facts or a distinct test-only numerical success algorithm.
   Challenge full validators with own positive/negative controls. Reproduce
   genuine original observations in new private outputs when needed to close
   instrumentation/parity questions; never rewrite source expectations.
2. Inspect original f32/order/SIN65536/rawbit import/start63291/boundary behavior,
   Cave/extra-Cave/Canyon112/strict thickness and no child recursion, replaceability/
   mask-before-write/barrier/tip-rescan, mycelium/no-rule/fluid edges. Verify
   actual direct lazy provider uses the same evaluator and memo as public Stored;
   temperature shares memo, nonconsumption issues no query, only rule column/
   block contexts reset. Aquifer caches and stale Lava flags persist throughout.
3. Independently audit all pre-operation charges against structural proofs:
   R63912717682; bind90; Noise321566836740286847972889004023138800132439;
   prepare117744178367 inside total; remaining
   321566836740286847972889004023021055954072; selector18716174375;
   rules45120000. Recheck constructor/297columns/315misses/945P and closed tree
   bounds against implemented loops. No corpus maximum substitutes for admission.
4. Inspect every private wide error through constructor/basis/RNG/Cold/Keys/
   Spline/density/Marker/ChunkState/preliminary/climate/Aquifer/selector/lazy rule
   path, public regression and exact limbs. Own targeted probes must exercise
   64/128/192 carry/overflow/refused charge, segment changes/history continuity,
   N−1/N/N+1 constructor/total/remaining and bind89/90/91. No truncation, hidden
   reset, narrow fallback or public cap upgrade. Inspect the finite-f64 cache
   repair including finite bit preservation, sentinel/type admission and400B
   public rule session. Preserve every old expectation.
5. Inspect exact Ready→Running→Complete/Poisoned states, prepare/refusal order,
   all individual/combined mismatches and defensive failures. Verify every
   output-refusal probe starts actually Published, ends Empty, and unchanged
   input remains Published; no reuse after poisoning. Add own independent
   boundary/refusal probes where existing tests only mirror implementation.
6. Native evidence must account complete simultaneous producers/input/output,
   constructor peak, borrowed/owned/static/tree/Arc capacities and scratch;
   partial825488B is not whole memory. Author full requested-heap observations
   are4175577B retained/5388857B peak, distinct from production value layouts.
   Verify allocation interceptor collision/tombstone/exhaustion controls and
   actual first invocation without warmup. Independently execute cold Debug/
   Release success/refusal probes, zero allocations AND deallocations with
   exactly4096B positive control; constructor separate. Cover all seven author
   probe categories and RR10-H1–H5, M13/M14/M15 and literal10816/406592.
7. Verify full final command argv/log/binary/source identities and outcomes.
   Do not repeat already green whole consumer suites merely for green output.
   Run fresh meaningful focused independent checks/probes in your private
   combination; broaden only for changes, failures or concrete open concerns.
   Explicitly distinguish author's47e checks, exact current-main combination
   and own probe overlays/results. Complete byte/membership audits before and
   after every overlay, preserving failed attempts and final immutable inputs.

## Exclusive output and verdict

Write only new `docs/agent-tasks/GEN-05_11_OVERWORLD_CARVERS_RUNTIME_REVIEW.md`
and `.codex-tmp/gen05_11-runtime-independent-review-r1/**`. No Git, agents,
production/dependency/lockfile/docsource changes outside that new report,
shared builds, author evidence mutation or historical fixture repairs. Do not
touch running GEN-06.3/GEN-03.1 correction scopes. Root owns integration.

Return CLEAN_RUNTIME_REVIEW only if every binding objection is closed, or
FINDINGS with stable IDs/severity/exact symbols/reproductions and requested
corrections. Distinguish numerical equality, accounting/errors/native behavior,
source findings and residual gates. Freeze complete new reviewer sources,
evidence/input pins and report, reverify all unchanged inputs and outside files.
Report exact paths/hashes and any failed/superseded attempts.

This cold producer does not establish warm/thread-history parity, practical
all-seed timing, actual generator-to-Chunk/biome/heightmap/light/persistence
integration, features/structures/Nether/End, Survival, plugins or App activation.
Natural-world activation remains a separate required operator milestone.
