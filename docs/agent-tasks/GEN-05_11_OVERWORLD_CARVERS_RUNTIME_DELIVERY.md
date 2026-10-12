# GEN-05.11 — cold Overworld Carver runtime delivery

12 October 2026. Author implementation and validation are complete for candidate044.
Independent runtime review and Root's integration decision remain pending. This
delivery does not self-accept the runtime or activate any App consumer.

## Exact candidate and isolation

The complete World candidate contains362 files. Its canonical path→SHA256 source
digest is `ab77a66d73a92882b661398931a99c6b7190984a1c49d99a13ac01586d735f19`.
`candidate-044.json` records the complete membership and per-file hashes. It was
announced before final checks, and shared/private World bytes stayed immutable
through those checks. Four pre-existing shared World Python bytecode files are
preserved outside source membership; private overlays exclude them.

Final checks used only the genuine readonly accepted-main archive at
`.codex-tmp/gen05_11-runtime-root-base-storage47e/archive`, main
`47eadc4c14813c201184b552a87ca7b9c6f97db4`, with only the complete World overlay.
Its1490-file `base.freeze.json` SHA256 is
`58ba13d6418133bb49a154682ba1d82533f63a5e6184c6c2a8b07c6c15a47a75`.
All324 accepted World files equal the original4225 baseline. All1166 accepted
outside-World files, including93 accepted Storage files, were byte/membership
audited before and after every final command. Consumer-generated `.codex-tmp`
artifacts were hash-recorded and relocated to `final-044/generated/consumers`
before restoring exact outside membership. No accepted source exemption exists.

The writable combination is `.codex-tmp/gen05_11-runtime/build-storage47e`;
the separate target is `.codex-tmp/gen05_11-runtime/target-storage47e`.
No shared mutable-WIP build was used. Original archive/check records remain
preserved, with old-base checks treated as supplemental author evidence.
No Git, dependency, lockfile, Registry, Storage, Sim, Session or App changes
were performed by this implementation. The whole delta is38 new World files,
38 modified World files and0 removals, plus this new delivery and private
runtime evidence. All202 pre-existing World fixture/reference files are
byte-identical to accepted baseline (`source-delta-044.json`).

Binding authority pins remain unchanged and are checked over full bytes:

| Authority path | SHA256 |
| --- | --- |
| `docs/agent-tasks/GEN-05_11_OVERWORLD_CARVERS_REVISION2.md` | `ddd6c13188cfffc570ad80df9b9678d534e7c04859f23cfd6ee9e97f003ecbec` |
| `.codex-tmp/gen05_11-r2/freeze-manifest.json` (448 files) | `46a5d1dc02e913b0a72c0a979b1ddd393f46b52504b13871ab00f21e88f3f267` |
| `docs/agent-tasks/GEN-05_11_OVERWORLD_CARVERS_REVISION2_REVIEW.md` | `2c9a9c7af235ccbaa5bf0ea72e5465b8885d3d34a29707f264130382bb472d0a` |
| `.codex-tmp/gen05_11-r2-independent-review/freeze-manifest.json` (181 files) | `2c7a2527b3173f933342b7db70fe0f0c36adcfe6fa72cc2b020c6d32e7da4f26` |
| `.codex-tmp/gen05_11-r2/input-pins.json` | `d4c9a8c60bf8ad0bac9d6f1d0c3fa8f0f8884c50c52a35caf4a045ed2921651c` |

The original task/contract and both patch layers remain binding wherever
revision2 does not replace them. The corrected direct-biome and cumulative-meter
provisions are implemented; obsolete Stored substitution/per-call reset do not
return. The accepted original archive authority is
`300784c05ead0b2a4c1a92b5e8d42a5c01b35a386ee1f3d1a432ae8ca559d72e`.

## Production implementation

`carver` exposes checked private-field `SeededCarvers`, a fresh owned
`PreparedCarverRun`, borrowed `CarverInput`, own `CarvedChunkBuffer`, one-shot
`carve_chunk`, exact `CarverReport`, typed errors, budgets and Rustdoc. Prepare
admits binding/producer identity, provenance, geometry/domain, native values
and all work allowances before producer construction. Carve first clears output
publication, preserves exact refusal precedence, then enters
Ready→Running→Complete or Poisoned. Only successful full-target execution
publishes; independently Published input Surface remains unchanged.

The original accepted .3–.10 producer cores supply Paper roots, immutable
MultiNoise tree, original zoom, temperature/rules, material and preliminary
surface facts. Every289 source biome queries execute X-outer/Z-inner before
the constant carver list is used. Consumed lazy topMaterial fix-ups interleave
with that source/carver history. A concrete private Stored/DirectCarver provider
shares the existing rule evaluator core: direct climate/selector/zoom executes
only at the existing biome memo miss, temperature shares that memo, and NoRule
and nonconsuming paths issue no query. Temporary disjoint field borrows avoid
self-reference or trait-object callbacks. Fix-ups reset only rule column/block
context; Noise, Aquifer, selector, RNG and cache/flag histories remain continuous.

Fresh Aquifer begins with scheduling flag=false, retaining location/status
caches and stale Lava flags through boundaries. Private derived halo and direct
quart-Y−17 remain unclamped, with public coordinate/cap constructors unchanged.
Geometry preserves original seeding, Cave/extra-Cave/Canyon execution, f32
arithmetic and SIN lookup, uniform upper1.0, Canyon112, child thickness1 and no
child recursion. Ellipsoid/skip/mask/write order, target-only writes, reached
surface, barrier mask/no-write, replaceability and live WORLD_SURFACE_WG rescans
are preserved. Output fluid marks follow the declared set policy; original
ordered duplicate postprocessing observations remain in the immutable fixture.

The complete65536-entry SIN static is a mechanical import of genuine rawbits,
262144B, with no libm generation or hand transcription. Raw little-endian bits
SHA256 is `cdfaec6870788e193dff3f1ddfa3ca7d3f1a897042d78365a1fe3a433be3627d`;
original SIN TSV SHA256 is
`6e16407bd7721398a296fb2a247ed2cb779e81fa997e1272d8d501c0e836d8ba`.
Import/fixture receipts and the full Rust membership test pin the import.

## Work authorities and exact faults

| Authority | Binding requirement / unchanged public cap |
| --- | --- |
| Carver geometry/provider |63912717682; public1..=2^36 |
| Carver bind census |90; public1..=65536 |
| Noise total including prepare |321566836740286847972889004023138800132439 |
| Prepare reserved inside total |117744178367 |
| Noise remaining admission |321566836740286847972889004023021055954072 |
| Selector |18716174375 |
| Rule work |45120000 under existing45908736 |
| Binding/run value caps |4096B each |
| Output owned native cap |448KiB |

Total and remaining exact little-endian limbs are `[117840306519,0,945]` and
`[96128152,0,945]`. Full total is never reserved twice. Constructor receives the
admitted private meter before its first charge and checks actual prepare used
against its reserved bound before Ready publication.
`allowance-record-044.json` extracts both exact limb arrays from Rust, checks
reserved+remaining=total and records all authorities alongside corpus maxima.

The internal unsigned192 authority has checked three-u64 limit/used, bounded
segment bookkeeping and pre-operation charge. Segment changes never reset
producer caches, epochs, RNG, selector leaf, Aquifer flag or history. Exact
private faults propagate through basis construction, metered RNG, Cold/Keys/
Spline, Perlin/Normal/Blended, shared spline evaluator, density/MarkerEvaluator,
ChunkState, climate/preliminary, Aquifer and lazy selector/provider paths.
Public adapters/counters/budgets/errors keep their accepted contracts; private
faults do not truncate through a public narrow error.

`tests/reference/carver-runtime/CHARGE_AUDIT.md` maps every new actual charge to
corrected §5: constructor recurrences,297 preliminary columns,315 cache misses,
945P bounded RNG draws, selector and rule/geometry bounds. C11 unmetered Legacy
geometry rejections and earlier immutable-router binding shuffles retain their
accepted scopes. The wide finite period proof is accounting, not a timing bound.
No bound/cap/scope deviation was introduced.

## Replay, boundary, refusal and mutation evidence

Both full World profiles execute actual Surface→Carver production replay for
every71 full Paper group. S1 is compared first, then every FINAL state/digest,
mask, live height, fluid set, source/start/RNG/carver projection, every Aquifer
result/scheduling flag,355 lazy fix-ups and exact source/fix-up block/quart/
biome chronology. The source query census is20519=71×289;42117 Aquifer calls
are compared. Both refused corpus classes remain typed Empty and natural edge
groups remain full. Vanilla provides25 discriminating full groups.

`replay-release-044/stdout.log` exposes all71 successful actual reports;
`counters.json` records their complete counters and totals. Maxima over this
corpus are geometry157933, Noise905681 (including construction), selector974373,
rules11088, starts86, writes3718, mask bits5065, fix-ups168 and Aquifer calls3815.
These are measured corpus values, not replacement structural bounds.

Tests cover all63291 historical start tests, the full SIN table and every
scripted float boundary, including uniform upper1.0 and Canyon112. Original
lower-quart and selector-history tie probes remain labelled controlled inputs.
Source-based edge controls cover mycelium, NoRule/nonconsumption, fluid fix-up,
mask skip,−56/−55,312/−62, barrier mask/no-write, stale Lava flag, shared
temperature memo and tip rescan; they add no synthetic natural-world oracle.

Actual Rust admission tests exercise N−1/N/N+1 total, actual constructor and
remaining requirements, bind89/90/91 and public0/1/cap/cap+1. Private64/128/192
carry, overflow and refused-charge controls compare full history/cache/epoch/
RNG/flag state across segment changes before source, inside Aquifer and lazy
fix-up. Primitive and nested tests check exact limbs in every fault adapter;
no producer reset is used to satisfy the tests.

Individual and combined refusal matrices verify exact precedence, no input
mutation and actual Published→Empty transitions. Defensive execution faults
poison the run permanently. RR10-H1 tests production build_chunk at y319;
H2 separates Lava from water in eroded/frozen paths; H3 pins literal10816/
406592 and all new layouts; H4 externally measures Binding/Material/Session/
UnclassifiedState refusals; H5 starts with genuinely Published outputs.

Private exact accepted47e+candidate044 clones kill actual M13/M14/M15 with
test assertion failures, exit101 (`rr10-mutants-final044/result.json`). A
separate mechanical audit compares all1528 restored clone files, not merely
the mutated driver (`rr10-clone-exact-final044.json`).

## Native values and external cold-process observations

Pinned toolchain: Rust/Cargo1.99.0, x86_64-unknown-linux-gnu, LLVM23.1.1,
GCC16.2.1 and glibc2.44 (`toolchain-final044.json`). Standalone production
rlib probes, independently pinned in both profiles, agree on these values:

| Production type/category | Bytes |
| --- | ---: |
| SeededCarvers / PreparedCarverRun |1560 /1360 |
| CarvedChunkBuffer value / report |1104 /104 |
| CarverInput / work budget / bind budget |16 /8 /4 |
| CarverError / CarverErrorKind |104 /104 |
| Prepared state and five small error enums |1 each |
| NoiseChunkSession / selector / rule session |704 /80 /400 |
| Surface columns / Surface buffer values |576 /1088 |
| Private cumulative meter / SampleWork |56 /64 |
| DirectCarver provider / Aquifer |48 /144 |

Full simultaneous cold constructor census retains input producers, prepared
invocation and output together. Both profiles observe4175577B/2638 live
requested heap objects after BUILD, and5388857B/5084 objects at peak. BUILD
performs656374 allocations requesting65116356B and650459 frees. Native observer
SHA256 is `a6bb420425fa5c4c0fa22d9dd03b73963229d99994dd4c8029dbd7549e3bd0d9`.
Its32768-slot full collision/tombstone/peak/live tests pass and bounded
exhaustion exits90 (`native-ledger-final044/result.json`).

Separately reported categories are output-owned418896B, Surface-borrowed406592B,
columns-borrowed10816B, binding1560B, rule binding19216B, template6622B,
temperature2232B, selector tree1181618B, Arc header16B/tree refs1, Noise
graph-admitted797304B/cache-admitted40528B/scratch-requested9216B/preliminary-
requested24576B/router Arc refs2/definition-map entries70, Canyon stack1536B,
tunnel depth2 and static SIN262144B. Admission categories overlap the full
heap census and are not additive heap partitions; the partial825488B output+
Surface sum is not a complete memory claim.

Each profile launches seven independent external cold-process probes: Carver,
complete native census, RR10, temperature, Stored biome, rules and Surface.
Each pins a copied binary and measures the genuine first prepared invocation
without an unmeasured warmup. Every DRIVE phase observes0 allocations,0
requested bytes and0 deallocations. Every positive CONTROL observes exactly
1 allocation,4096B and1 deallocation. Carver measures21 cold successes,
21 actually Published→Empty refusals and13 constructor refusals per profile;
its BUILD is65702 calls/8234072B/64860 frees. RR10's BUILD is3148 calls/
578835B/3140 frees; its DRIVE also observes zero for all four refusal families.

Stack-write observations are separate: full native BUILD/DRIVE Debug656920/
476328B and Release92152/26744B; Carver BUILD/DRIVE Debug643304/476328B and
Release85544/26744B. These are observed stack writes, not full stack reservations
or RSS bounds. Heap observations count requested bytes, excluding allocator
metadata. Native census binaries contain test observers; production layouts
are separately checked without those fields.

## Complete final commands and results

Every Cargo command ran from the private combination's `server` directory,
using `/home/jstrauss/.cargo/bin/cargo`, `CARGO_TARGET_DIR` set to the separate
Storage47e target and `PYTHONDONTWRITEBYTECODE=1`. Test/build/doc/Clippy commands
used `--locked --offline -j 2`.

```text
cargo fmt --all --check
cargo test -p ferrumc-world --locked --offline -j 2
cargo test -p ferrumc-world --release --locked --offline -j 2
cargo clippy --workspace --all-targets --locked --offline -j 2 -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-world --no-deps --locked --offline -j 2
cargo build --workspace --locked --offline -j 2
cargo test -p ferrumc-storage -p ferrumc-sim -p ferrumc-anvil -p ferrumc-session
  -p ferrumc-testkit -p ferrumc-bench -p ferrumc-app -p ferrumc-plugin-block-rules
  -p ferrumc-plugin-region-guard --no-fail-fast --locked --offline -j 2
```

Exact argv/environment/elapsed-time/log hashes are in `final-044/results.json`.
The consumer command above is one argv, line-wrapped here. Both World profiles
have415 passed,0 failed,7 ignored across10 suites. All seven ignored allocator
tests are explicitly driven by the seven fresh-process probes in each profile.

| Check | Exit | Passed tests | Ignored | Log SHA256 |
| --- | ---: | ---: | ---: | --- |
| fmt | 0 | 0 | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| world-debug | 0 | 415 | 7 | `e6301fd49c61dfddd30aae76fd1bb3cc5d0f63e7d4363d1a04baa77ffe5f3859` |
| world-release | 0 | 415 | 7 | `f81ef4c181c54092ead708890070676c3aa55e256639aa9f8901125a770d74af` |
| clippy-workspace | 0 | 0 | 0 | `77430f4b2f2c61a1ab851201e02d59bde008d795e254453adb754a9c9a7fe72b` |
| rustdoc-world | 0 | 0 | 0 | `efd846ef12218a94468ee45066ce956a80ec01b7038e73759055de1ced159a7d` |
| build-workspace | 0 | 0 | 0 | `9e1bbf5ba35c572dd12d5e11a5960d4f86016528a0df95e03e8569db1328a9ff` |
| consumers | 0 | 1550 | 1 | `7d86ae79e55b94e390e889915207f29b882ac36f2abc88e0778f1f690d5c25f7` |

`final-summary-044.json` mechanically summarizes complete successful logs and
all14 external probe measurements. `final_probes.py`, `public_layout.py`,
`replay_report.py`, `rr10_mutants_final.py` and the native ledger runner retain
their exact commands and copied binary hashes. Additive fixture verification
passes in normal and Python−O mode (`fixture-durable-*-044.json`), checking
membership in pinned original freezes, not hash equality alone. Complete
authority audit passes (`authority-complete-final044.json`) for448/181/427/433
historical freezes,1133 original inputs, original JDK/JAR membership/shadow
guards and accepted World membership. Public claim scans use the accepted
scanner without invoking Git and record no new finding after this delivery.

## Preserved attempts and review package

Every failed/superseded log and receipt remains under the runtime area. In
particular, candidate040 exposed accepted .9 rule-session layout drift408 vs400.
The repair uses a private eight-byte finite-sample cache, preserving exact finite
f64 bits and historical400B session/cap expectations. No old numerical or native
expectation was rewritten. Historical layout/cap tests and a cache bit/layout
control pass. Candidate042 stopped on a test-only helper dead_code warning;
candidate043 passed both World/probe profiles but strict Clippy found one missing
Rustdoc backtick. Candidate044 changes only that comment from043 and reruns all
final checks. Superseded mutant042 clone restoration refusal remains preserved;
the fresh044 whole-clone audit passes independently. Tooling mistakes and their
corrected attempts are also retained, including initial bytecode membership,
historical-vs-overlay pin, helper lifetime and claim-scanner attempts.

New readonly delivery package paths, relative to the repository:

- `.codex-tmp/gen05_11-runtime/delivery/source/archive/`: all362 candidate files.
- `.codex-tmp/gen05_11-runtime/delivery/source.freeze.json`: exact source manifest.
- `.codex-tmp/gen05_11-runtime/delivery/evidence/archive/`: all new receipts/logs/
  tools and pinned probe binaries, including failed/superseded attempts.
- `.codex-tmp/gen05_11-runtime/delivery/evidence.freeze.json`: exact evidence manifest.
- `.codex-tmp/gen05_11-runtime/delivery/document/`: byte-identical delivery copy.
- `.codex-tmp/gen05_11-runtime/delivery/freeze-manifest.json`: complete package
  membership plus source/evidence/document manifest hashes.
- `.codex-tmp/gen05_11-runtime/frozen-reverification-044.json`: separate independent
  post-freeze byte/membership/readonly/outside-file audit and exact handoff hashes.

Freeze is performed only after required checks and the post-delivery claim scan
pass. Private build/target and restored mutant archive copies, generated Python
bytecode and the package itself are excluded from evidence replication; original
copies remain preserved. All frozen bytes and source/outside memberships are
independently reverified. The package manifest avoids a cyclic document hash;
Root receives its exact SHA256 with the handoff.

Validation is for this cold corpus and pinned process/toolchain. It establishes
neither arbitrary warm-history equivalence nor live-server performance, latency,
TPS, RSS or production activation. Caller-trusted preliminary-corner provenance
retains the accepted producer contract. Features/ores, structures, lighting,
persistence/identity, Nether/End, plugins and Survival are outside this runtime.
Root assigns independent runtime review and decides integration; natural-world
consumer and activation tickets remain separate work.
