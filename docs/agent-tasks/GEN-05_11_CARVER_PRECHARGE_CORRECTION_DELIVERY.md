# GEN-05.11 — RR11-M1 climate precharge correction delivery

12 October 2026. Author implementation response for Root; independent delta
review and runtime acceptance remain pending. GEN-06.2 remains queued.

## Correction and exact subject

`NoiseChunkSession::climate_sample` now evaluates the original f64 root into a
local, admits its existing conversion charge, then performs the sole original
f32 cast/store and finite check. A refused conversion charge executes zero
conversions. Successful expression order, bits, producer history, public caps,
private unsigned192 limbs, budgets and typed errors remain unchanged. No public
API, production type layout, dependency, lockfile or consumer integration changes.

The private tested baseline is the genuine accepted b1798614 archive1509
(324 World), base manifest
`5435d74ecac890aaeb838f9d1886a14427943a52ce611f0071873c00654e7dcf`, plus
the whole frozen044 World and frozen044 delivery. The virtual1547 correction
subject manifest is
`67788a1fb33ce2c87f1e6fe1e759ca7c7e0ddce28ce90a523aa3f256db277240`.
Private final membership1549/outside1185 remains exact. Shared Registry/lockfile
work was recorded separately and never adopted. Publication12fb changed only
five docs; subsequent Root publication5602fa20 changes only the implementation
board and GEN-06.3 Revision3/review-assignment docs. Authorized live shared doc
drift is recorded separately and does not replace this build authority. No Git
or agents were used.

Original044 World digest
`ab77a66d73a92882b661398931a99c6b7190984a1c49d99a13ac01586d735f19`,
source manifest
`243418db1406675450902b17a4f38c280b48a093faa7f91232e0cc2d1bb4334f`,
author package
`574e54d8dafbd087d238d3d349f83369457664dec157e461ec2179129ce8c162`
remain unchanged, including original fixtures and expectations.

Final whole World364 canonical path-to-hash digest is
`09b837b7478c4ade7fc4abb0ab376d193a8ed817fd7c2c89e1e28c77ac66fd53`; candidate manifest
`72df2e26961e6900b99e5f6112459ee966a51d8621779f98bce72f331d915beb`. Root was notified at this whole readonly candidate before
its final checks. Earlier Root-verified f3050ce0 remains preserved under
`superseded-f3050ce0/`; one test-helper borrowing correction addresses its strict
Clippy failure. Its production bytes are identical to the final candidate.

Exact old-to-new five-file delta and all old/new hashes are in `candidate.json`
and `source-delta.patch`: modified `src/density/chunk.rs`,
`src/density/chunk/tests.rs`, `src/carver/tests.rs`; added
`src/density/chunk/tests/precharge.rs`, `src/carver/tests/precharge.rs`.
Only `chunk.rs` changes production behavior. A cfg(test) fixed thread-local
observer reads bits immediately after the actual cast/store; it supplies no
answers, modifies no meter and adds no production session field.

## Executed actual regression and defensive order

The review's exact seed0/Overworld request is
`CarverClimateQuart::source(ChunkPos::ORIGIN,-8,-8)`: source chunk offsets
(-8,-8), derived quart(-32,0,-32), physical(-128,0,-128). It must not be
relabelled as physical quart(-8,-8). Every control constructs a genuine fresh
router session; constructor work570916 is asserted. Allowances39/40/41 execute
0/1/1 casts, first bits3d8704f4;59/60/61 execute1/2/2 casts, second bits3dc4fc7f.
Each refusal has exact Noise ProducerWorkExceeded required[1,0,0] and full
limit/used limbs. Poisoned requery returns exact Lifecycle without advancing
history or meter. Complete-query work345 is observed;344/345/346 controls
prove last-conversion refusal with five casts versus successful exact six bits
and history. Successful bits are
`[3d8704f4,3dc4fc7f,3dceb90c,bd06b6ab,3f4077e2,bf0b45d1]`.

Controlled actual evaluator constants test public and private defensive order,
separately from natural answers. Finite f64::MAX costs one root unit; allowance1
refuses conversion with zero casts. Allowance2 admits the cast to f32 infinity
and returns the original temperature NonFiniteCoordinate with used2. Old044
could return that numerical error without paying for conversion; the correction
deliberately makes admission precede this error. Infinity/NaN roots yield
NonFiniteDensity after one admitted root unit, before conversion. Public
allowance0 preserves nested Density(WorkExceeded), while refused conversion
preserves top-level WorkExceeded. Raw-core controls do not claim wrapper
poisoning. Both final-candidate profiles execute all three new regressions.

The exact final-candidate private old-order mutation moves only the charge back
after the actual cast/store/observer/finite check. Debug and Release compile and
exit101 at the allowance39 assertion: actual conversion1, expected0. Defensive
overflow assertions also fail. This is an assertion-level mutation kill, never
runtime acceptance. `mutation-final-clippy-fixed-old-order/` retains full
overlay manifests, changed sources, logs, results and actual failing binaries.

## Complete required validation

All commands ran in the fresh exact private `build/server`, dedicated `target`,
with `PYTHONDONTWRITEBYTECODE=1`. Cargo test/Clippy/doc used
`--locked --offline -j2`. Both World profiles passed418 tests with0 failures and
7 ignored allocator probes across10 suites;361 passed/6 ignored are unit tests.
The focused final Release regression passed3. Required commands were
`cargo fmt --all --check`, complete World Debug and Release,
`cargo clippy -p ferrumc-world --all-targets -- -D warnings`, and World
`cargo doc --no-deps` with `RUSTDOCFLAGS=-D warnings`.
`final/results.json` preserves complete argv/environment/timings/log identities.

| Final check | Exit | Log SHA256 |
| --- | ---: | --- |
| fmt | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| world-debug | 0 | `53e14a9d78641ca424d3874ddd87110da3cc4617e51ac77c03db1e9763f44c83` |
| world-release | 0 | `c955a2d585915959f93280d3e1716f12b20c92a7234b08ab6a8b8ddee0bc1b23` |
| focused-release | 0 | `82c0e9044c3cc87fdd3dd909c6bee58e6088f6dd7398dbe4e6e5aad30942c20d` |
| clippy | 0 | `7ecb048c95315edee30a43f2b0f01a416bd8f3b4fcca3d41b8710a14dec1b672` |
| rustdoc | 0 | `67dad82d9cd1826f9adbd81ce273cbee26aa7baf80448dce97af8daf08074b31` |

## Fresh affected cold numerical and native evidence

Both profiles freshly execute the exact final-candidate cold Carver allocation
probe:21 successes,21 already-Published output refusals,13 constructor refusals.
Both run complete simultaneous native census and complete71-group composition
replay from their copied pinned binaries, without warmup. The two typed refused
groups remain Empty. Aggregate is
`GEN0511_REPLAY [71,42117,355,157933] Vanilla-different=25` (spacing as in logs).
All71 full Paper groups and25 distinguishing Vanilla labels remain unchanged;
historical fixture expectations were not regenerated. The full composition uses
real Surface, climate/selector/zoom, all289 ordered source queries and actual
interleaved lazy consumed fixups, Noise/Aquifer/rules histories and geometry.
`cold-{debug,release}/` retains commands, measurements, logs, copied binaries and
external interceptor binaries.

| Profile | Final-candidate test binary | Old-order mutant binary |
| --- | --- | --- |
| debug | `009f78dee9570b1b0237f80d2e1506d2b12a39dae73911e04460bebd67c72464` | `2e0151f459d0f77eb7cea25c4760c32a00a14613ae19b7a7cf4dfe7110a13792` |
| release | `dd15c52e762e1a6d9d5174c52fd4a257f3d752cf69054eb0dc8139c9ff6e5978` | `a7f9db4d1ab87162a266b04b7acc8d167b02558ba2ef644d60ea908df822937d` |

Each external allocator CONTROL observes exactly one4096B allocation and one
free. Prepared DRIVE observes zero allocations, requested bytes and frees.
Both final profiles reproduce native BUILD retained4175577B/2638 objects,
peak5388857B/5084 objects. Static SIN262144B and production value/stack layouts
are separate categories; capacities are not counted twice. These are requested
Linux/glibc allocator bytes for pinned cold binaries, not RSS/latency/TPS or
all-seed performance. Stack-write values retain the tool's limited scope.

The additionally executed instrumented climate overlay at f3050ce0 is reused
explicitly: Debug/Release nine genuine fresh cold queries39,40,41,59,60,61,
344,345,346; two conversion refusals, five other refusals, two successes and
seven poisoned requeries. Both have zero DRIVE allocations/bytes/frees and the
positive4096B control. Constructor BUILD42899274 requested bytes/468462 calls/
458299 frees is recorded separately. Its sole extra ignored-test registration
and actual observer checks are retained in `climate-allocation/source-overlays/`
with complete manifests and actual binaries. This is instrumentation-only
evidence from f3050ce0; final candidate differs solely in a defensive test-helper
reference lint fix. It is not presented as an unmodified final binary.

## Source audit, preserved authority and evidence reuse

`ANALOGOUS_CHARGE_AUDIT.md` records the explicit12-category audit of roots,
conversion, cache/publication/store charges, Cold/Keys, markers/fills/drivers,
Aquifer, RNG/basis/noise/spline, selector, reserved rule calls and Carver.
No additional concrete defect of this same precharge obligation was found.
Separate paid root evaluation and later cache publication were distinguished;
all affected publication/stores follow their separate admission. Existing
finite-before-store validation and pre-child cache history policies stay
explicit; no numerical precedence or budget policy was silently changed.

Full original assignment/contract/reviews, Revision2 correction/response/CLEAN
preparation review, runtime assignment/delivery/review and AGENTS/CLAUDE/
PAPER_REFERENCE/World invariants/affected accepted GEN-05.3–05.10 contracts
remain binding. Relevant real Java implementations, callees, base/applied
sources and feature patches were read. `reference-equivalence.json` pins137
reference sources and records base/applied correspondence; unrelated Moonrise
server integration is not claimed. The complete runtime review SHA is
`78f2fcb70b6fa2936ffe2ee4fcff352e2cafe33686d529291f1454fa3a35b29a`,
review392 manifest
`5f1d3ea6f8af96e7587361739f19bfd16970216c1b51d66f1e02bf96a1ad8cf5`,
9-member final seal
`b928e0df3d9c287104fc24860deb0d3f6ad4345e1f00d81212766ca8f0e04224`.

Initial/final authority audits freshly hash all448/181/427/433 historical
freezes,1133 original inputs and JDK/source/JAR/shadow memberships. Integrity
audits verify all1509 baseline bytes, virtual1547 subject, original362 source,
original945/review392/seal9 packages, restored1538 reviewer private files,
all architecture/binding/reference/instrumentation pins, final364 source and
private1549/outside1185 bytes. Four exact inventoried preexisting shared pycache
exclusions are retained unchanged. Concurrent shared outside drift is recorded
explicitly; immutable private baseline/reference/history remains exact.

Unaffected frozen044 evidence is reused, with original source/package pins:
genuine Java exports U/F/R/D/observers; SIN65536/52-member tag/period/bound
controls; validator semantic mutants; RR10 M13/M14/M15 and cache/ledger controls;
unchanged production layouts; temperature/Stored/rules/Surface/RR10 allocator
categories. Fresh World layout/error suites and affected final cold probes are
listed above. Historical1550 consumer passes belong to47e, not freshb179.
Consumer suites were not repeated because no API/layout/consumer change occurred.
The7 ignored probes are not all claimed freshly executed here; unaffected five
allocator categories use their explicitly identified frozen evidence.

All failed and superseded attempts remain preserved: initial wrong log location,
helper/module/type errors, wrong public0 error assumption, initial Release
regression failure, integrity path-resolution error, and f3050ce0 strict test
lint failure. Green diagnostic violations of old order are not acceptance.

## Freeze and residual gates

Readonly `delivery/source/archive/` contains the entire364-member final World;
`source.freeze.json` pins exact membership. `delivery/document/` contains this
byte-identical new document; `evidence/archive/` contains explicit new evidence,
attempts/tools/overlays/pinned binaries; `evidence.freeze.json` pins membership.
`freeze-manifest.json` pins the complete package without a self-hash cycle.
Private builds/targets and duplicate source archives remain preserved outside
replication and are accounted for in the exclusion receipt. A separate final
verification seal hashes all frozen members, readonly state and immutable inputs.
Exact final package/document/evidence hashes are supplied to Root separately.

CR11-B1/B2 closure remains preparation-level; RR11-M1 is this implementation
response, pending independent newly frozen delta review. No author acceptance,
cap upgrade or App activation occurs. Warm/thread-history parity, practical
all-seed timing, generator-to-Chunk/biome/heightmap/light/persistence integration,
features/ores/structures, Nether/End, Survival, Java plugins and equivalent-scope
performance remain separate requirements. Caller-trusted preliminary-corner
provenance retains the accepted contract. GEN-06.2 and natural-world activation
remain queued for their required acceptance and integration decisions.
