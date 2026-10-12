# GEN-05.9 — overworld surface rules runtime delivery

Runtime delivery, 12.10.2026. World-only implementation of the accepted
[contract](GEN-05_9_SURFACE_RULES.md) (SHA-256 `1906f57e52d3b7745b1a2aae2322eaa36caa3658c9a00bd6e98e5cc592a20e6f`)
under the [runtime authorization](GEN-05_9_SURFACE_RULES_RUNTIME_TASK.md)
(SHA-256 `69e40ca73ffcf6b379b06a70699141d428d63111893ff9f200033b12c62adad2`) with its Root decisions, conditions A1–A8 (§13), the
policies P1–P6, the review hints R3-H1–H3 and the carried GEN-05.8 condition
R-A1.

Written: `server/crates/ferrumc-world/**`, `.codex-tmp/gen05_9-runtime/**` and
this document. No Git operation, no other agent, no dependency, lockfile,
generated file or other crate was touched. The contract, its three reviews,
the work order, the frozen oracle `.codex-tmp/gen05_9/**` (freeze manifest
SHA-256 `9f09cbd5bd191f3e173bf1843ab45fc5b5b9411de350fe45ff6a4429a8fa0e11`, all
458 entries and the whole 11,176-file tree re-hashed before and after: tree
digest `86e28de36e53023fde2a20621783ab7c5366bf6eb41ba31860c92ed73dd931f8`
unchanged) and every historical GEN-05.x artefact are unchanged.

## Basis and isolation

- Root's readonly Git archive of `main` `e18efd22c9e062190b023575ac53fb0da04c2eb8`
  (1325 files, `base.freeze.json` SHA-256
  `2deaad94edf10780dd4f5431631408f4a883410257565058cd4c5d889b0eb3f3`) was
  byte-verified and copied to `.codex-tmp/gen05_9-runtime/work`. Only the World
  crate was changed there; every build and check ran on that copy with
  private target directories. The shared tree's Storage (DUR-03.3) and
  Session (NET-04.2e2) WIP was never built or read.
- Before installation the shared tree's `ferrumc-world` equalled the base byte
  for byte; the only extra files are the four pre-existing untracked
  `__pycache__/*.pyc` interpreter caches, which were left as they were.

## Implemented contract

New public module `surface_rules` (`pub mod surface_rules` plus module docs in
`lib.rs`, A7):

| Item | Content |
| --- | --- |
| Budgets | `SurfaceRuleBuildBudget` `1..=262_144`, `SurfaceRuleBindBudget` `1..=65_536`, `SurfaceRuleWorkBudget` `1..=67_108_864` (`InvalidBudget` otherwise) |
| Template | `SurfaceRuleTemplate::compile_builtin(NoiseSettingsResourceId, budget)`: preorder node table (428 nodes), separate typed tables for states, biome sets, thresholds, gradients, Y/water/stone checks, noises and random names |
| Binding | `SeededSurfaceRules::bind(Arc<SurfaceRuleTemplate>, WorldSeed, SurfaceNumericProfile, PaperFlatBedrock, SurfaceRuleBindBudget)` |
| Session | `SurfaceRuleSession::new(&rules, &resolver, &temperatures, PreliminarySurfaceCorners, SurfaceRuleWorkBudget)`; `begin_column`, `surface_depth`, `evaluate(SurfaceBlockInput, &WorldSurfaceHeights)`, `min_surface_level`, `work_used` |
| Values | `SurfaceRuleOutcome::{State, NoRule}`, `SurfaceColumnPos`, `SurfaceBlockInput`, `WorldSurfaceHeights`, `PreliminarySurfaceCorners` |
| Errors | `SurfaceRuleError` (kind, block, node, limit, requested, maximum; 88 bytes, no message), `SurfaceRuleErrorKind` with all 22 contract kinds, `ResolverMismatch::{Seed, Settings, Source, Geometry, ZoomSemantics}`, `SurfaceRuleLimit`, `SurfaceRuleRefusalCategory::{Policy, Limit, Input, Defensive}` |

All fields are private, there is no `unwrap`/`expect` outside tests,
`forbid(unsafe_code)` is unchanged and every public item has rustdoc. The
module docs state the profile: Paper build 60 numerics, Paper zoom semantics,
Paper default flat-bedrock mode.

### Compile

One checked parse of the Registry settings definition with World's existing
definition JSON admission, the original `Codec.DOUBLE`/Gson INT token decoding
of the density compiler, one preorder walk and separate reference passes.
Refusal order: census allowance, settings admission (P3), source limit, JSON
admission, settings fields (`legacy_random_source`, `sea_level`,
`noise.min_y/height`), codec shape (`UnknownCodecType`, `MalformedDefinition`,
`InvalidAnchor`), references (blocks, biomes, noises, random names), census
caps, pinned tree census (428 nodes, 202 references), retained bytes. The
bedrock gradients compile to `paper_optionally_flat_bedrock` mechanically.

### Bind

P1, then P2, then the bind census, all before any sampler: positional root
`Xoroshiro(seed).forkPositional()`, nine `NormalNoise` converted with the
crate-private `paper_build60_basis` (system noises `surface`,
`surface_secondary`, `clay_bands_offset` plus six tree noises), two gradient
factories and the 192 original clay bands (`generateBands` draw order).

### Evaluate

Explicit stack of depth ≤ 32, no recursion, no heap. Per-column caches
(threshold nodes, steep with the four heights it read, hole, secondary,
minimum) and per-block state (biome memo, temperature, per-block nodes)
follow the original laziness; the column draw is `f64::from(next_double() as
f32)`; Java `int` wrapping, `Mth.floor/lerp/map`, JDK 21 `Math.round` and Java
`%` for the band index are kept (a negative index is a defensive `Noise`).

### Work accounting (A3, R2-M1, R3-H1)

`begin_column` 2 units, the column minimum 1 unit once, `evaluate` admitted
only with `467 + m` remaining (467 is computed from the template: 287 rule +
141 condition nodes + 33 threshold samples + 1 secondary + 1 band + 2 draws +
1 biome + 1 temperature). P4 precedes work. `Poisoned` is the first check of
every fallible method; `work_used` is infallible and readable after poisoning.
`WorkBudgetExceeded` never poisons and has no effect.

## Original evidence

### Fixture (A1, A2)

`tests/fixtures/surface-rules-original-1.21.8/` holds the 14 round-3 fixture
files byte-unchanged (manifest digests checked by a test).
`tests/reference/surface-rules/` holds byte copies of the frozen helper,
runner, guards, projection, input-pin preparation and fixture generator, the A5
extension and the allocation runner, with a README.

`surface_fixture.py validate` against the frozen raw output, normal and `-O`,
on the installed fixture: identical output in both modes and byte-identical to the frozen round-3 `fixture-validate-normal.json`: 38,891 expanded rows equal the raw rows, the H8 invariants hold, all 38,889 replay rows are re-derived (G 2490/4230/0, C 5478/24,626/2065) and 3529 steep reads are consistent. The clay fixture validates in both modes (3393 columns re-derived).

### Replay (A1)

Real GEN-05.5 arrays: all 720 `createBiomes` arrays of the 80 groups (cold
history per group, nine chunks row-major) equal the original digests. Every
group gets a real `SurfaceBiomeResolver` with Paper semantics and its real
binding (6 bindings over 4 seeds and 3 settings).

| Tier | `full` | `partial` | `refused` |
| --- | --- | --- | --- |
| G | 2490 | 4230 | 0 |
| C | 5478 | 24,626 | 2065 |

- `full`: one shared session per group, `begin_column` once per run, rows in
  ordinal order. Result, surface depth, biome (resolver and memo), the complete
  `computed` trace including 2455 noise rows and 7968 draw rows with bits,
  singletons, 871 steep-height reads, 2776 minimum surfaces, 4 secondary values
  and 2776 column cells are equal.
- `partial`: a new session per row; result, surface depth, biome, the
  temperature singleton, the per-block `computed` subset and the draw bits are
  equal; per-column cells are evidence only.
- `refused`: a new session per row; exactly the expected kind (2059
  `BlockOutsideGenerationHeight`, 6 `InvalidStoneDepth`), no outcome and an
  unchanged state projection (active column, surface depth, every per-column
  cache with minimum and steep heights, per-block state, `work_used`). No
  original value of a refused row is compared.
- Heights view as declared: `final` from `groups.tsv`, `steep-probe` from
  `steep-heights.tsv`.
- The compiled table equals `tree.tsv` before and `tree-paper.tsv` after the
  substitution for all three settings; the difference is exactly
  `paper-tree-substitution.json`. Census, bands, resolved anchors, sea level
  and generation context of all 80 groups are equal.

### Paper basis (A5)

Node-wise and noise-wise against every non-refused fixture cell, Debug and
Release: all 33 threshold nodes and all 7 threshold noises (surface 2333,
powder_snow 1260, packed_ice 326, ice 258, calcite 200, gravel 178,
surface_swamp 21), the surface column noise and column draw (2776 each) and the
secondary noise (41) equal the Paper basis bit for bit; the Vanilla basis
matches 0 of them. All 48,685 recorded gradient draws are equal.

**`clay_bands_offset` (direct bit comparison, R3-H2).** The extended copy of
the frozen helper (`.codex-tmp/gen05_9-runtime/clay-oracle/`) adds one
side-effect-free read per observed genuine column:
`SurfaceSystem.clayBandsOffsetNoise.getValue(x, 0.0, z)` of the group's genuine
`SurfaceSystem`. Inputs are the frozen pins (`input-pins.json` `fdfb8b24…`);
the extended helper (`69ac0750…`) and the runner copies are pinned in
`extension-pins.json`.

- Normal and Python `-O` runs: exit 0, byte-identical to each other. All 23
  data outputs equal the frozen round-2 raw outputs byte for byte (only the
  compiled helper classes differ, plus the two new `clay.tsv`).
- Guards: 16/16 refused in both modes; both `guards.json` are byte-identical to
  the frozen round-3 ones (`838ecefb…`, `ac03d213…`) and differ only in
  `entrypoint_python_optimize`.
- Column set (mechanical): the distinct `(group, ctx_x, ctx_z)` of the Paper
  G/C rows of the frozen raw `paper/rows.tsv` whose `biome_direct` is
  `minecraft:badlands`, `minecraft:eroded_badlands` or
  `minecraft:wooded_badlands`: 3393 columns. `clay_fixture.py` generates normal,
  `-O` and from the `-O` run byte-identically; `validate` (normal and `-O`)
  regenerates and re-derives the set on a second route.
- Pass criterion: every listed column has runtime Paper-basis bits equal to
  the genuine Paper bits and Vanilla-basis bits equal to the genuine Vanilla
  bits; any deviation means A5 is not met.
- Result: Paper 3393/3393 equal, Vanilla control 3393/3393 equal, genuine Paper
  and Vanilla bits differ in 3393/3393 columns (discriminating). A5 is met.

## Policy, limit and malformed tests (separate from original equality)

- P1/P2: `Vanilla` → `UnsupportedNumericProfile`, `Enabled` →
  `UnsupportedFlatBedrock`, profile before flat bedrock, both before work.
- P3: `caves`, `floating_islands`, `nether`, `end` → `UnsupportedSettings`.
- P4: `y` −65/−64/319/320 and `i32` extremes, stone depths 0/1/4096/4097 above
  and below, water `Some(i32::MIN)`/`Some(i32::MIN + 1)`/`Some(i32::MAX)`/`None`,
  in the order y, above, below, water, without effect.
- P6 and consistency (A8, R2-H5, R3-H3): `Seed` (seed 1), `Settings`
  (`large_biomes` template), `Source` with real arrays from `Fixed(plains)`,
  `End` and `MultiNoise(minecraft:nether)`, `Geometry` with real arrays of
  `(-80, 416)` and `(-64, 400)`, `ZoomSemantics` (Vanilla); combinations show
  the order. The level-height assumption `(-64, 384)` is named in the
  constants, rustdoc, INVARIANTS and the test.
- Budgets at both domain ends; compile census 133,777/133,778/133,779 and bind
  census 14,756/14,757/14,758 (58 octave slots + 56 × 259 + 192 + 3; the
  existing `SampleWork` charge of the nine conversions is exactly
  58 + 56 × 259).
- Work matrix (§8, every row): `begin_column` at 1/2/3 and domain before work;
  `min_surface_level` no column, uncached 0/1/2, cached 0 (remaining 0 with no
  active column is positioned by a private `cfg(test)` control, because the
  public API cannot reach it); `surface_depth` 0 units; `evaluate` cached
  466/467/468 and uncached 467/468/469; P4 at remaining 0; `Poisoned` first for
  every fallible method; no effect of every refusal; restart in a new session
  equals an uninterrupted column (outcomes, trace, `work_used`).
- `SURFACE_RULE_OVERWORLD_CHUNK_WORK` = 45,908,736 evaluates all 98,304 blocks
  of one chunk without refusal.
- A8 steep timing: a view changed after a column's first steep test keeps the
  first result; a refused call before the first steep test fixes nothing, the
  first admitted test does. Both tests use a genuine row whose outcome differs
  between the two views.
- Malformed definitions: JSON syntax and all JSON limits, unknown codec types,
  extra/missing fields, wrong shapes, number ranges, anchors (±2031/2032),
  multipliers (±20/21), strict block states, unknown biome/noise, invalid
  random names, and the refusal order shape → blocks → biomes → noises →
  random names → census caps.
- R-A1 (A6): `stored_biome/tests/halo_order.rs` pins the two-phase halo order
  on X+, X−, Z+ and Z− edges; production `stored_biome` is unchanged.

## Native layout and allocation (A4)

Measured retained bytes, pinned as exact test constants (Debug = Release):
template 6622 (design cap 16 KiB), binding 19,216 (32 KiB), session 400
(4 KiB). `NativeLimit` refuses at N−1 and admits N and N+1 through the real
admission for all three, and for the eight census caps at the real census
(nodes 428, depth 13, sequence children 12, biome references 53, biomes per
condition 3, noises 9, random names 2, states 24). Synthetic compiles cover
511/512/513 nodes, 63/64/65 children, 255/256/257 and 15/16/17 biome
references, 15/16/17 noises, 7/8/9 random names and 127/128/129 states; the
compiled depth is bounded by the JSON depth (30 and 31 compile, 32 needs JSON
depth 33 and is refused as `JsonDepth`). No cap was changed.

External probe (the unchanged GEN-05.7 `allocator_probe.c`, SHA-256 `1e7a61bd…`)
on the exact final unit-test binaries in fresh processes
(`tests/reference/surface-rules/measure_allocations.py`):

| Phase | Debug (`3e6a7a97…`) | Release (`00912fff…`) |
| --- | --- | --- |
| CONTROL | 1 call, 4096 bytes, 1 free | 1 call, 4096 bytes, 1 free |
| BUILD (1 compile + 1 bind, counted) | 3341 calls, 638,395 bytes, 3248 frees | identical |
| EVALUATE (38,889 sessions, `begin_column`s and `min_surface_level`s; 36,824 evaluations, 2065 refusals) | 0 calls, 0 bytes, 0 frees | 0 calls, 0 bytes, 0 frees |

Observed stack-write high water: BUILD 136,488 bytes (Debug) / 21,688 bytes
(Release); every other phase stays below the probe's 16,384-byte marker
margin. Error 88, outcome 8, block input 20, heights view 1024 and corners 16
bytes. These are allocator observations for the named binaries only.

## Interpretations, stated openly

- **Checked JSON parser visibility (reported to Root as a contract question).**
  §3 requires "World's existing checked JSON parser", which is `pub(super)` in
  `density`. The runtime makes `density::json::{parse, Value, ValueKind, Stats}`
  and `density::compile::{number, integer}` `pub(crate)` and re-exports them
  crate-privately in `density.rs` (two `pub(crate) use` lines). No behaviour,
  signature or public API changed. The alternative was a second JSON/Gson
  implementation.
- **Census allowance before settings admission.** §7 lists "budget, settings
  admission" first. The compile requirement (`bytes + JSON nodes` from the
  Registry statistics `+ 428 + 202`) is known before parsing and is checked
  first; a non-overworld setting with a sufficient allowance is
  `UnsupportedSettings`.
- **Compiled depth cap.** With the JSON depth cap 32 the compiled depth cannot
  exceed 31; the `Depth` limit is exercised through the real admission at
  12/13/14.
- **Defensive structure check.** A rule node of condition kind (unreachable for
  compiled templates) is refused as `MalformedDefinition` with its node and
  poisons; the poisoning tests use this private corrupted control, because
  every biome/temperature/noise failure is unreachable for admitted inputs.
- **`SurfaceColumnPos::new`** returns `None` for local coordinates ≥ 16,
  like `LocalBlockPos`; `from_block` is total.
- **Strict admission.** Unknown fields, lenient `Properties` and non-string
  property values are refused (FerrumC admission policy); the builtin data
  does not use them.
- **Session centre.** `new` re-checks the nine halo keys with the accepted
  `admit_chunk` (unreachable for a complete resolver; mapped to
  `Biome(CoordinateDomain)`).
- **INVARIANTS.md** gained a surface-rules section and one sentence in the
  NoiseChunk section naming the binding as the second Paper-basis consumer
  (A7).

## Mutation check

`mutants/mutants.py` applied 18 temporary source mutants to the private work
copy and restored every file by hash (`results-round1.json` SHA-256
`310b2c28…`). 16 were killed in round 1:

- column draw without f32 rounding; Vanilla instead of Paper basis; steep axes
  swapped; gradient chance not inverted; minimum surface without −8;
- evaluate pre-check one unit short; P4 admitting `y = 320`; resolver source
  before settings; column domain before `Poisoned`; threshold caches kept
  across `begin_column` (killed by the shared per-group replay session); hole
  `< 0`; Paper substitution disabled; secondary depth rounded;
- single-phase surface halo (R-A1, killed only by the new `halo_order` test);
  `begin_column` charging 1 unit; bands from a wrong named fork.

Two survived round 1 and led to sharper tests before the candidate:

- **M12, temperature at sea level 64.** No fixture row reaches the altitude
  adjustment (`y > sea + 17`). The new controlled test
  `temperature_condition_uses_the_settings_sea_level` scans the frozen-ocean
  hole columns at `y = water − 1` (81..200), compares 28,560 temperature results
  with the GEN-05.7 table at sea level 63, and requires discriminating cases
  (165 where 64 would differ).
- **M16, per-block state reset by a refused call.** Publicly unobservable, but
  it breaks "no mutation before a refusal"; the P4 test now also compares the
  per-block state after an admitted call.

Round 2 (`results-round2.json` `6f2b9ed3…`) killed both. All 18 are killed and
the sources were restored by hash.

## Final checks (unchanged candidate)

Candidate `candidate-01.source.json` (SHA-256
`3fe6f959ed437a1fc1bd11afe7e9272e7619283744a84ad5e5fe0af6dba08291`, 291 World
files: 46 added, 7 modified, 0 removed, 0 outside World) with a readonly copy
`candidate-01-src/` was written before the checks. All checks ran on it
unchanged with the fresh target `target-final` (`final/checks.json` SHA-256
`fe2ec2a68301297c43fca6908cab3aa8015ebf7f304da8770be58d1d82c19197`):

| Check | Command | Result |
| --- | --- | --- |
| World Debug | `cargo test -p ferrumc-world --jobs 2 --locked -- --test-threads=2` | 360 passed, 0 failed, 3 ignored (allocation probes) in 9 suites: lib 304 + 2 ignored, integration 46 + 1 ignored, doc 10 |
| World Release | same with `--release` | identical counts |
| Clippy | `cargo clippy --workspace --all-targets --jobs 2 --locked -- -D warnings` | pass (all 33 workspace crates) |
| fmt | `cargo fmt --all --check` | pass |
| rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-world --no-deps --jobs 2 --locked` | pass |
| Workspace build | `cargo build --workspace --jobs 2 --locked` | pass |
| World consumers | `cargo test -p ferrumc-storage -p ferrumc-sim -p ferrumc-anvil -p ferrumc-session -p ferrumc-testkit -p ferrumc-bench -p ferrumc-app --jobs 2 --locked --no-fail-fast` | 86 suites, 1426 passed, 0 failed, 1 ignored; `multiplayer_visibility` passed, no rerun |
| Allocation | `measure_allocations.py`, Debug and Release | as above |

Of the World tests, `surface_rules` contributes 38 passed tests and 1 ignored
probe, `stored_biome/tests/halo_order` 2.

Audits before and after (`final/audit-{before,after}.json`) are identical in
every check: the archive still has 1325 files equal to the base; the 1080
outside-World work files equal the base (the app tests left only their known
`.codex-tmp/p57-plugin-testhost-*` artefacts in the copy root); the work World
equals the candidate; the shared-tree World still equalled the base; the frozen
GEN-05.9 evidence (458 freeze entries, 11,176 files) is unchanged.

After the green checks: `source.freeze.json` (SHA-256
`dd65ad0ea07967d624376b58e9e7b37a3e3a5457798fa0300741679ad2a8a3eb`, 291 files)
and `evidence.freeze.json` (`ec67684473f90b083fa922017d2724d18cf820007e3d272029365b04a05ab30f`,
105 files). Only then was the World crate installed into the shared tree
(`install-report.json`): the 291 installed files equal the source freeze, and
nothing else in the shared tree was written apart from this document.

## Non-claims and open points

- No TPS, RSS, latency or full-server claims; no Survival, Java plugin or
  natural-world activation claims.
- No `SurfaceSystem` column driver, chunk write, eroded-badlands or iceberg
  extension, carver or feature; the evaluator consumes an explicit context.
- `PaperFlatBedrock::Enabled` stays refused; there is no valid evidence for it
  (B1).
- The steep-timing requirement remains source-based; the A8 tests are
  controlled tests, not original evidence.
- The allocation and layout figures apply only to the named binaries and
  toolchain.
