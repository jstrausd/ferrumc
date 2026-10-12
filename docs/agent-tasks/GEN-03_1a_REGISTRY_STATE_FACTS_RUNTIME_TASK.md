# GEN-03.1a — Registry-only original state facts, Rust implementation

Root implementation assignment, 12 October 2026. Implement the Registry split
of GEN-03.1 Revision2. This is an implementation authorization for one crate;
acceptance of the delivered runtime still requires independent review. The
Heightmap/Ore Query53 finding does not block this separate producer, and remains
open for the future World/Ore adapter.

## 1. Binding authority and Root disposition

Read completely, including their findings, responses and residual obligations:

- `GEN-03_1_BLOCKSTATE_HEIGHTMAP_ORACLE_TASK.md` and
  `GEN-03_1_BLOCKSTATE_HEIGHTMAP_ORACLE.md` (original contract SHA256
  `260ff459932d8f82f0c5c2bc30898b0de5b0e2a324939911ecafd155a45ee08f`).
- `GEN-03_1_BLOCKSTATE_HEIGHTMAP_ORACLE_REVIEW_TASK.md` and
  `GEN-03_1_BLOCKSTATE_HEIGHTMAP_ORACLE_REVIEW.md` (r1 review SHA256
  `a45abd91edfad2c5fb5f7d33312e1e5a3b217daf9a09ecd52827e7b46b2a1213`).
- `GEN-03_1_BLOCKSTATE_HEIGHTMAP_REVISION2_TASK.md` and the complete
  `GEN-03_1_BLOCKSTATE_HEIGHTMAP_REVISION2.md` (SHA256
  `469933e39a0cd41e6374ae9f66479cecb702cd56419d6c30f4e7d4d855bd292e`).
- `GEN-03_1_BLOCKSTATE_HEIGHTMAP_REVISION2_REVIEW_TASK.md` and the completed
  `GEN-03_1_BLOCKSTATE_HEIGHTMAP_REVISION2_REVIEW.md` (SHA256
  `f9578c75df327866d7de9e9759fd6d012dacc1a93b0b39adff3f75ceb98906c0`).
- AGENTS, CLAUDE, PAPER_REFERENCE, Registry INVARIANTS, relevant architecture
  and ADRs; the full affected original Java methods, callees, base patches and
  feature patches using the immutable reference lists in the author package.

G31-R1-B2 and M1 close only at preparation/tool level. G31-R1-B1 closes only
at design level and does not grant a World implementation in this task.
G31-R2-B1 remains open: same real Query53 kernel on the existing parent meter,
bounded Chunkselection and a separately corrected Ore debit plan are required.
The previously proposed World 2048/512/4608-byte targets, reservations and
HeightPreparation2^23 remain unaccepted by this assignment.

Root accepts only these Registry design bounds for measurement and enforcement:
65536 bytes for all newly emitted static facts/metadata/alignment, 131072 bytes
for the complete new simultaneous build phase, domain cardinality26, and the
Revision2 bounded join/stream algorithm and conservative u64 work ceiling
7391501569. These are limits, not measured sizes or time/performance evidence.
The existing loader/emitter and borrowed normalized block metadata remain
separate additional inventories and phases. No runtime allowance is raised.

Root verified the complete author and review freezes, including exact lstat
membership and immutable inputs. Author freeze SHA256
`f49d4664fdb31f6d690e233250591912b0b71494d90b0878a5a99ce930144e5f`;
review freeze SHA256
`bdea7cfd73cb7bbf983bd60656a13578deaeb01e8ebb8db3efe97ff4699b7300`.
Intake: `.codex-tmp/orchestration/gen03_1-revision2-review-intake.json`.

## 2. Exact baseline and exclusive scope

Use genuine accepted main `65c7f682bf1dc7bdc58263aec226cd8cec5f3c3b`:
`.codex-tmp/gen03_1a-registry-root-base/archive`, 1505 regular files, including
47 Registry and 324 accepted World files. `base.freeze.json` SHA256
`7917226aff23edc427743b73ee8451ae11b3db87faeb11fb2c26178ba1afb2ec`.
Verify every baseline file and membership before and after work. Copy the archive
to a new private directory under the evidence scope, then overlay only the entire
Registry candidate for checks. The shared World is an unaccepted frozen Carver
candidate and must never become the build baseline.

Exclusive writes:

- `server/crates/ferrumc-registry/**` including its build code, fixture/reference
  additions, tests, README and INVARIANTS.
- `docs/agent-tasks/GEN-03_1a_REGISTRY_STATE_FACTS_RUNTIME_DELIVERY.md`.
- `.codex-tmp/gen03_1a-registry-runtime/**` for private copies and evidence.

No Git, other agents, shared builds, dependency/lockfile changes, other crate
changes, old fixture/freezes/tool changes or App/Flat consumer activation.
Root is the only integrator. Do not reserve a World writer or import pending
Ore/Carver code. Record complete outside-Registry before/after byte inventories.

## 3. Required implementation

Implement all Registry requirements of Revision2 sections2–3 and relevant
section8/9 acceptance requirements; those exact contracts govern the details.
Whole blocks pin admission precedes the existing loader. After write/drop of
the existing block emission String and temporary indexes, borrow the normalized
metadata and run the new phase without another DOM or metadata clone.

Use only the bounded u16[1105] block-name index, deterministic insertion sort,
complete original property/domain identity and checked forward mixed radix.
Preserve true,false boolean order and original enum/integer domain order. Join
all27946 original identities, defaults and flags; bitmap and exact closure;
20 complete local fluid identities. No second full-state identity index, default
repair, bounding-box predicate approximation, collision hash retry or fallback.

Vendor unchanged genuine inputs with complete original origin/provenance and
fixed whole-file pins. Expected facts must come from the independent original
outputs, never from this generator. Parser/hasher/join/stream work is bounded
and charged against the reviewed u64 blueprint, including head/end/cleanup/
publication paths. An unmetered library hash or sort does not establish that
contract. Keep error precedence and bounded typed error representations.

Generate one static facts file through the reviewed bounded stream and exclusive
part/flush/close/rename publication. Reserve cleanup before creation, retain old
output until successful rename, no retry loops or fallible computation after it.
If a platform/Rust close or IO capability requires a concrete contract correction,
report the exact gap and smallest correction; do not silently weaken it or claim
an injected wrapper models a tested real IO path.

Provide opaque CheckedStateId, fallible u32 conversion, six closed predicate
kinds and immutable allocation/IO-free fact getters. No public raw/default/
serde/unchecked authority. Existing World raw IDs remain raw; no Flat changes.
These facts confer no collision/light/support/network-fluid-ID or world-history
authority. Preserve the dependency-free runtime and all existing registry data.

## 4. Actual evidence and delivery

Run actual malformed/length/row/field/domain/identity/default/flag/fluid/duplicate/
missing/extra tests with the real parser; whole genuine all-ID comparisons;
private test descriptors distinguish pin drift from malformed-input admission.
Test exact authority entry types and membership before payload reads; never open
FIFO/socket mutation stimuli or follow their links. Execute genuine positive
controls, failure precedence, short IO, write/flush/close/rename and cleanup
refusals, allocation/native admission failures and unchanged old output checks.

Capture literal emitted native static size, actual owned capacities, headers,
temporaries, stack/moves and borrowed metadata separately. Models are not actual
measurements. Enforce the limits with meaningful N-1/N/N+1 controls. If claiming
runtime zero allocations, measure real first cold use and refused conversions
in fresh Debug/Release processes with a positive4096-byte allocator control.
Build allocation is permitted within its actual complete bounded inventory.

In the private exact archive overlay, run complete Registry Debug/Release tests,
all-target Clippy `-D warnings`, fmt and rustdoc `-D warnings`, locked/offline/
jobs2, dedicated target. Appropriate consumers must be checked when generated
output/API changes require it; do not repeat green unaffected whole suites.
Cargo: `/home/jstrauss/.cargo/bin/cargo`. Preserve failed/superseded attempts.

Notify Root at complete immutable candidate before final checks; freeze complete
Registry source plus delivery and explicit evidence only after all required
checks pass. Reverify whole baseline/outside/historical bytes. Provide exact
manifests, SHA256, measured inventories, executed tests and remaining gates.
End with a concrete result or exact blocker, without waiting/polling for Root.
Natural-world activation, Survival and whole FEATURES remain outside this ticket.
Java/Paper plugins are not implemented by this producer. No runtime acceptance,
RSS/TPS or performance improvement follows from this delivery.
