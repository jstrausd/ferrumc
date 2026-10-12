# DUR-03.3 — Independent Storage Runtime review, round 3 (candidate round 6)

12 October 2026. **Verdict: FINDINGS.** B1 is corrected; R2-H1 and R2-H2 are
corrected in the reviewed runtime paths; R2-H3 is accurately disclosed and remains
unchanged. No new authority, CAS, replay, atomicity or persisted-grammar defect was
found in the round-6 delta. **R6-B2 remains a concrete startup/liveness finding and
an unresolved acceptance gate**, rather than being waived because it is old or
because the author documented it. Root owns integration and any explicit limit
acceptance. This review grants no App activation.

I am the independent reviewer, not the candidate author. No Git or other agents
were used. Only this new document and
`.codex-tmp/dur03_3-runtime-root-review-r3/**` were written. All compilation and
probes used a private writable copy of Root's immutable archive with a new private
target. Original freezes, production, author history and read-only bases were not
edited. Instructions, Storage invariants, architecture/storage, ADR-0004/0007/0009,
Paper reference, the original complete design/source review/Root contract review,
runtime reviews 1/2 and the complete Delivery responses were read.

## Basis and evidence integrity

| Input | Independently checked SHA-256 / result |
| --- | --- |
| Original immutable contract | `2edfc588fd8d7e96753f6beb4bb7c2ad50a064e4c266681ab5ff94550c4342ab` |
| Original design freeze | `3710c57265084f9a6cd59d48fc77b9340fcd255bd0f235545c7ca639b92de92f`; all 16 artifacts unchanged |
| Original accepted archive manifest | `c148936b29cc58d7ac25b2904618523c3b3a163476a2bf380b1d00fadf07c7b0`; all 1,224 files unchanged; 461 dependency, 45 redb and 82 Paper whole-file pins unchanged |
| Current Root base.freeze.json | `41ae7f504d2001445e5c43e07b985eeecb29d444469b3e4b72258407b2d58583`; all 1,468 archive files unchanged, no extra file |
| Candidate manifest file | `d21db90ec56958c965c2a8fb288d21b82ad5af22c8bda9cd0d471cd894abd0d7` |
| Recomputed candidate digest | `3c53838631108f43b76c689ebfd89dc2e3104901005ef1a9ae454d93039c6664`; all 91 files match archive, frozen candidate and observed worktree |
| Delivery | `e3248d523e549320b845e612aa8d4553a4a747d7ff05d3c26f5d0b56f2c8877d` |
| Author evidence freeze | `0335589a6f659d79a0750e53487d960356ba567138e6829f33ad8dc8f9b73e0c`; all 38 evidence files match |
| Cargo.lock | `183450e729d90bd6b5212bc9f6d351b3f58b950c30f2a03c15bcf0ed7829fe4a`; byte-identical in compared bases |

Root's current archive identifies accepted main
`c0b78c2361eec8c32152b125cb0602ea769ae9bc` plus the **unaccepted** frozen Storage
candidate and Delivery. The author metadata names `8da4c770222d7d40b747ce2abb7a08ff9706fa25`;
Root reports actual round-6 checked main `0c0fdb4d`. I did not infer either commit
with Git or treat them as the current accepted basis. `basis-comparison.json`
compares actual byte inventories: the author's isolated sources match its
metadata's original outside-Storage files and the 91-file candidate. Its only
extras are the reported two App testhost `.so` artifacts and Python `.pyc`.
Current Root adds/changes Session, Sim and World development; these are accepted
archive bytes here, never mutable World/Sim WIP. A selected 150-file transitive
Storage codec/core/math/items/registry/config/lock comparison is identical. World
export/density/surface and Sim/Session changes are separately inventoried; author
consumer/workspace results are **not** results for current main.

## Round-6 delta assessment

The exact round-5→6 diff contains ten changed files and one new integration test.
The persistent writer/apply code, immutable FCBQ/FCOB/FCBL/FCBR grammars,
17-table catalog, issuer, revision/incarnation, replay/CAS order, returned-error
preflight and uncertainty boundaries remain intact. The three opener changes
preserve primary errors instead of substituting scratch cleanup failures.

- `engine.rs:192–204` derives current sign size from UTF-8 byte lengths and chest
  size from all 27 `persisted_stack_size` results before encoding. These match the
  real versioned World and FCIS layouts (empty chest 301; blank sign 38). The new
  normal `ferrumc-items` dependency is justified, was explicitly authorized by
  Root, and adds no lockfile/external package. Its preflight is borrowed and
  validates component counts/IDs, NBT limits, lengths and registry counts with
  checked bounded accumulation; it does not normalize payloads.
- The 16-KiB bound is still the parser/encoder form limit. Unknown future entity
  kinds and refused stack forms reserve it conservatively. Sign field errors and
  over-limit aggregate chest forms refuse; their reservation is a bounded upper
  estimate, not a promise that an invalid encoding succeeds. No valid current
  entity retains the old flat 16-KiB reservation.
- `entity_bytes`, `entities` and `view_encoded_len` charge before encoding. The
  endpoint proof has two actual complete chunks, so its two map encodings are
  real separate work. The extra apply decode is now charged before decode.
  Legacy entity bytes are decoded at their input length and canonical reencoding
  is charged at its output length (28-byte legacy empty chest → 301). Legacy
  effects remain rejected; no raw-original authority or migration rule changed.
- `raw::frame_q` returns the same charged Q parse with its borrowed frame. Both
  `Lasts::load` and `player_last` reuse it. Removed `receipt_history` did only the
  now redundant parse and `validated`; the same identity, union, outcome,
  Player/Chunk header/postimage and Journal checks still run. The coherent reader
  lifetime, eight-slot runtime cache and budget sharing remain intact.
- `keep_cause`/`then_cleanup` run cleanup on both success and failure; an earlier
  error wins and the extra cleanup error is logged. All three opener failure
  sites use the helper. The real sabotage test covers init failure, scan failure
  and cleanup-only failure, preserving foreign replacement bytes. R2-H3's inode
  check→path unlink window is unchanged; disclosure is accurate, not a fix.

Earlier review protocol gates remain binding. Their previous green observations
are not replaced by code similarity or this narrower check. I found no delta
that weakens snapshot authority, full before/post binding, legacy bypass refusal,
shared Player revision/receipt namespace, old-incarnation exact replay, limits or
commit/error classification.

## Independent execution and author results

Own runs: current Root archive + frozen candidate, private `work/` and initially
empty `target/`, toolchain 1.99.0, `--jobs 2 --locked`, Debug. Logs and probe sources
are frozen under the review evidence directory. Shared fixture helpers supply
public constructors and independent table enumeration; the new probe byte models
are derived from the contract, not returned Storage requirements/private plans.

| Own check | Observation |
| --- | --- |
| Prior probe 5, unchanged logic | Memory/redb lookup at 10,242,913 bytes and replay at 11,949,995; N passes, N−1 refuses with no mutation; old request replay after reopen |
| Prior probe 6, explicitly adapted oracle | View term is fixed independent chest length 1,651, replacing 16,384; exact need **5,827,683**, N passes/N−1 refuses on redb; both backends preserve views, revisions, historical receipts, new scoped commit, exact replay and stale CAS. Removed charge = 64×(16,384−1,651) = **942,912** |
| New probe 9, mixed signs/chests | Four distinct entity-only gameplay histories, fixed lengths 38/46/301/1,651; own model **77,541**; N passes and N−1 refuses on **both** Memory and redb; coherent stamps/raw pairs/views, redb reopen unchanged |
| New probe 10 | Empty/truncated/unknown/schema/overflow/trailing/16-KiB+1 and readable legacy effect inputs refuse; uncovered effect and extra Keep-only union refuse; absent receipts and unchanged stamps/raw pairs/views, redb reopen unchanged; blind empty writer refuses |
| Candidate `player_block_bytes` | 4/4 pass on current basis, including shared/distinct receipts, exact mixed shared union and near-cap union |
| Candidate `player_block_contract` | 3/3 pass; independent actual 17-table fixture and complete Player/components/endpoints/reopen |
| Candidate `player_block_entity_bytes` focused | 4/4 pass: 4096 empty chests over 3-column snapshot/2-column commit, 1100 over 8-column commit, exact entity budget, form-bound refusal without mutation |
| Candidate effect-dense residual | 1/1 passes on Memory/redb; independently rechecked N=468,516,117 for two histories, N−1 refusal, three-history need=702,773,966>512 MiB; two-history maximal commit refuses without mutation, one-history commit succeeds (267.90 s Debug) |
| Original round-1 reviewer probes | 4/4 pass; operation namespace, three-way race whole loser state (1 joint win/15 losses), missing scratch retry, all 1..8 distinct-history unions |
| Candidate entity-length unit | 1/1 pass: nine current forms, 301/38/16,384 anchors, malformed/over-limit models and legacy canonical charge |
| Candidate primary-error unit | 1/1 pass: actual init/scan+cleanup double errors, cleanup-only error, foreign replacement preserved |

In total, **23 focused tests passed** in these runs. The prior-review test copy
retains one unused-constant warning (`COUNTERS`); it is outside the candidate.

The unmodified old probe 6's N−1 failure in the **author** log is not normalized
into a passing test: that oracle contains the removed flat view reservation.
My adaptation changes only that independent size constant; the delta is exactly
942,912. Similarly old probe 7's expected refusal is now false; the matching
admission/state assertions above establish B1's correction.

Own initial probe compile had a reviewer mistake (`dump` instead of existing
`raw17`); it never executed and is retained in `independent-initial.log`. The
corrected source/log is separate. No candidate source was changed to make a test
pass. Full green matrix was not repeated: author evidence and round-2 full checks
exist, and this review targets the actual delta and unresolved issue.

Author evidence, independently hash/log/source audited **rather than rerun**:
354/354 Storage Debug and Release in 18 result blocks; workspace all-targets
clippy/fmt/build and Storage strict rustdoc exit 0; consumers 844 passed,
0 failed, 1 Java test ignored in 58 blocks. Compiler logs show fresh Storage
compile/check/documentation, and source manifests before/after match. Detailed
log hashes are in `author-log-audit.json`. These remain author's earlier-basis
results, not current-main consumer acceptance.

## R6-B2 — startup amplification remains an acceptance finding

**Severity: medium; unresolved liveness/capacity gate to CLEAN_RUNTIME.**
Call: `RedbStore::open_with_player_block_durability` → `initialize` → `scan_with`.
Source: `src/redb_store/block_validation.rs:539–577`, especially `:564–569`.
For every scoped journal row, the scan fetches its receipt again and calls
`raw::frame`, which parses and hashes all Q. Runtime `Lasts` and the startup
128-entry proof cache do not deduplicate that journal branch.

For one receipt with n chunks, Q bytes and m effects, R = Q + 71 + 9n. This branch
alone charges exactly

`m × (16 + R + Q) = m × (87 + 9n + 2Q)`

relation bytes, **plus** outer Journal key/value scan, all other tables,
Receipt/Chunk/Player relations, endpoint proofs and scratch-index validation.
This is actual work, so reducing its accounting without eliminating the work
would be incorrect.

| Allowed shape / own calculation | Journal receipt-read/hash charge alone |
| --- | --- |
| Own probe 8: n=3, Q=2,425,362, R=2,425,460, m=1024, no entity blobs | **4,967,258,112 bytes** (4.626 GiB) |
| n=1, Q=2 MiB, m=1024 | **4,295,065,600 bytes** (>4 GiB) |
| Author's maximal odd toggle form: n=1, Q=33,488,850, R=33,488,930, m=1019 | **68,250,374,124 bytes** (63.563 GiB), arithmetic independently checked |
| Algebraic Q=32 MiB, n=8, m=1024 | **68,719,639,552 bytes** (>64 GiB); bound calculation, not an executed exact-shape fixture |

Original hard shapes remain exactly 1..8 chunks, 0..4096 distinct guards,
0..1024 ordered effects, Q≤32 MiB, each full/overlay≤3 MiB, Player≤1 MiB,
effect entity≤16 KiB; union/endpoint/authority constraints still apply. Nothing
in the persisted agreement reduces m when Q is large. Runtime validation bytes
are capped at 512 MiB; startup relation bytes have their own positive configurable
u64 bound and no 512-MiB ceiling. A committed valid form can therefore be admitted
at runtime but refused on exclusive reopen under the **same configuration**.

Own Public API probe 8 demonstrates this without corrupting storage: image-CAS
seeds three noisy full images, then one legitimate 1024-effect alternating chain
returns column 0 to its original endpoint; raw-equal Replace proposals cover
columns 1/2. The commit, lookup and exact replay succeed. After dropping the
handle, both 1-GiB (same as create) and 2-GiB relation allowances refuse with
`ScanCapacity("bytes")`; every actual logical row in all17 tables remains equal,
and the scratch directory is empty. Reopening with **16 GiB** succeeds; the
original receipt is byte-identical, the old-incarnation request returns Replayed,
views/raw pairs match, and all17 tables remain unchanged. The complete probe
passes (513.51 s Debug); this is a deterministic diagnostic run, not a throughput
benchmark.

The exact admissible set is not expressible as just “up to 1024 effects” under
any fixed small startup allowance L: at minimum the entire database must satisfy
`Σ m_i×(87+9n_i+2Q_i) + other_actual_scan_work ≤ L` along with raw row/byte,
relation-cell and scratch limits. Before other work, probe 8 allows at most
floor(L/4,850,838) journal rows: **221 at1 GiB, 442 at2 GiB**. For Q=2 MiB/n=1,
the upper bounds are **255/511**. These are necessary upper bounds, not promised
admission thresholds; retained history adds cumulative costs and never expires.

**Classification against the original contract:** the contract explicitly allows
bounded cumulative startup scan refusal and separate local limits; this is not
an atomicity, corruption, or changed-format violation. Nevertheless the runtime
acceptance gate includes meaningful limit and real close/reopen tests. The author
newly documents a valid committed form that cannot reopen under customary fixture
limits and avoids reopen in its effect-dense test. “Old” and “documented” do not
resolve that operational limitation. Root's instruction to document B2 is honored;
it is not interpreted as an independent CLEAN_RUNTIME decision or App approval.

**Proposal, without production edits:** in the Journal pass keep one borrowed
last receipt row plus `raw::frame_q` result for the current operation's contiguous
range. On a new operation, charge/read/frame once; on following rows reuse it and
validate each scope, operation, ordinal, sequence and exact effect against the
retained Q. Preserve the global contiguous-sequence check, both relation
directions, receipt-range nonoverlap and whole-receipt endpoint validation.
Malformed interleaving or repeats outside the bound range must still refuse; a
cache may never become authority. Only one bounded borrowed receipt is retained;
no unbounded receipt map or reset scan budget is needed. Add independent real
reopen N/N−1 tests for small-effect large-Q, near32-MiB entity effects, multiple
operations and corrupted cross-operation/ordinal/range relations.

If Root accepts this as a temporary API limit instead of fixing it before
integration, explicitly record the formula, measured same-limit failure,
configuration/recovery obligations and cumulative retained-history restriction.
Assign **DUR-03.3b — bounded single-receipt reuse in Format7 startup Journal
validation**, with the tests above, **before Sim/App activation** (and keep
DUR-03.3a before DUR-05). This review does not create or authorize that production
ticket. Until that explicit disposition, verdict stays FINDINGS.

Additional low-severity note: `block_validation.rs:569` still calls `raw::q`
after `frame` without an additional byte charge. R2-H1 is fixed in `Lasts` and
`player_last`, but the documentation that every Reopen second parse is charged
is too broad for this Journal branch. It is bounded structurally and does not
weaken validation; `frame_q` reuse in the proposed B2 fix removes it as well.

## Remaining runtime limits and evidence boundary

The original 16,777,216-cell model still bounds a maximum snapshot at7,405,568
and commit at13,172,736; current probes exercise all eight independent histories.
Entity encodings alone no longer create B1: aggregate entity tails≤2 MiB make
snapshot entity-map encoding≤48 MiB and commit≤64 MiB. This does not guarantee
all large-history byte forms are admitted.

Distinct large receipts still consume real read/hash/image/effect bytes. At Q's
32-MiB bound eight distinct minimal receipt-read/hash costs alone exceed512 MiB;
for an eight-column caller including its fixed419 bytes the lower bound is
**536,872,603 = 512 MiB +1,691 bytes**. The author's +1,272 figure counts only
the eight receipt terms, which is consistent. Current after-images and endpoint
work increase costs: image-history snapshots can refuse at6, maximal image
commits at4 histories; dense form-bound effects can refuse snapshots at3 and
commits at2 histories. The first classes are shape-dependent arithmetic, not
new executed exact32-MiB fixtures. They remain explicit Root limit decisions,
not expanded hard limits or gameplay fallback authority.

Final assessment, source-after checks, own result/log/probe hashes and all new
review evidence are frozen in
`.codex-tmp/dur03_3-runtime-root-review-r3/final.freeze.json`. No RSS,
process-crash/power-loss, live Paper/client differential, full Survival,
Java-plugin, migration or full activation claim is made.
