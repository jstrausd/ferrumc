# GEN-05.10 — overworld surface column driver runtime delivery

Runtime delivery, 12.10.2026. World-only implementation of the accepted
[contract](GEN-05_10_SURFACE_COLUMN_DRIVER.md) (SHA-256 `bd69a4ea106655d752623df93b7c2d513572897dc89b714f1ba04be08d8830e3`)
under the [runtime authorization](GEN-05_10_SURFACE_COLUMN_DRIVER_RUNTIME_TASK.md)
(SHA-256 `05d68fef19e5377533032e22c1baede8447f4e8a733a97d6701b6ea0ad2b5c3a`): Root decisions D1–D5, the
conditions A1–A6 of the [review](GEN-05_10_SURFACE_COLUMN_DRIVER_REVIEW.md)
(SHA-256 `c5cf5ab5d12ed55f257ff0ce19f7015615480f273575cf35923ea27a39161538`) and the carried GEN-05.9 hints RR-H1/RR-H2. The
[work order](GEN-05_10_SURFACE_COLUMN_DRIVER_TASK.md) (SHA-256 `f9862c96…dfbf1`) stays binding.

Written: `server/crates/ferrumc-world/**`, `.codex-tmp/gen05_10-runtime/**` and
this document. No dependency, lockfile, generated file or other crate; no other
agent. The contract, review, authorization and work order, the frozen oracle
`.codex-tmp/gen05_10/**` (freeze manifest SHA-256
`ce9703069995f4dd947a22e61fa5d606918e9634b22478e9e0f19c824bd4887d`, all 321 entries re-hashed before and after the
checks) and every historical GEN-05.x artefact are unchanged. Disclosure: one
read-only `git diff --stat` ran once by mistake in the shared tree during
development (it only printed the existing WIP summary); no other Git command ran.

## Basis and isolation

- Root's readonly archive of `main` `9f842bc489fba47cfefc67ebad5af557112be429` (1393 files,
  `base.freeze.json` SHA-256 `ca81e729b42e421d80e8dea4e32d50ef0289645905c00eb4628e7a2b57a37bb1`) was byte-verified and
  copied to `.codex-tmp/gen05_10-runtime/work`. Only the World crate changed
  there; every build and check ran on that copy with private target
  directories. The shared tree's Storage WIP was never built.
- Before installation the shared tree's `ferrumc-world` equalled the base; the
  only extra files are the four pre-existing untracked `__pycache__/*.pyc`
  caches, left as they were.

## Implemented contract

Submodule `surface_rules::column` (no new `pub mod` in `lib.rs`), re-exported
from `surface_rules`; private fields, rustdoc on every public item, no
`unwrap`/`expect` outside tests, `forbid(unsafe_code)` unchanged.

| Item | Content |
| --- | --- |
| Budgets | `SurfaceColumnBindBudget` `1..=65_536`, `SurfaceColumnWorkBudget` `1..=1_048_576` |
| Binding | `SeededSurfaceColumns::bind(&SeededSurfaceRules, budget)`: six extension noises (Paper basis) and the closed alphabet; `seed`, `resource`, `construction_work`, `alphabet_len`, `retained_bytes` |
| Buffer | `SurfaceChunkBuffer::new()` (two boxes); `published()`, `state(BlockPos)`, `world_surface_heights()`, `fluid_flag(BlockPos)` return data only when `Published` |
| Driver | `build_chunk(SurfaceChunkInput::new(rules, resolver, temperatures, corners, material), SurfaceColumnWorkBudget, &mut SurfaceChunkBuffer) -> SurfaceChunkReport` (work used, max column work, session work, eroded/rule/frozen writes) |
| Errors | `SurfaceColumnError` (kind, chunk, column, block, limit, counts; no message), `SurfaceColumnErrorKind` with the nine contract kinds, `BindingMismatch::{Seed, Settings}`, `MaterialMismatch::{Seed, Settings, Geometry, Chunk}`, `SurfaceColumnLimit`, `SurfaceColumnRefusalCategory` |

Crate-private additions: `WorldSurfaceHeights::set` (private, H3);
`density::material_alphabet` (the GEN-05.4 `MaterialBlocks` resolution as a
read-only path, A5); a `cfg(test)` `ChunkMaterialSamples::with_states` control.
No GEN-04/.3/.4/.5/.7/.8/.9 behaviour changed.

**Driver (contract §3–§8).** Copy of the GEN-05.4 material, live
`WORLD_SURFACE_WG` (top non-air or −65), columns `lx` outer / `lz` inner,
`i4`, column biome at `(x, i4, z)`, eroded prepass, `i5`, one `begin_column`,
the original stone-depth / water-height loop (search to −65, lava sets the
water height, `evaluate` only on the exact default state), frozen postpass with
`min_surface_level()` and the stale `i4`. Writes stay in `[-64, 319]`, set the
fluid flag for fluid states and update the live heights with the original
`Heightmap.update` rule; the live view is passed to every `evaluate`. One fresh
GEN-05.9 session per chunk with `SURFACE_RULE_OVERWORLD_CHUNK_WORK`. Refusal
order before any column work: binding, material, session admission, driver
allowance; defensive failures leave the buffer `Empty` (D3).

**Conditions.**

- **A1:** `refused:CentreOutsideDomain` pins the GEN-05.5 halo refusal
  `ChunkOutOfRange { axis: "x", coordinate: 1875000 }` and the GEN-05.8
  `CoordinateDomain` at centre (1874999, 0) (with the centre's own genuine
  array, so domain precedes presence); `build_chunk` is never reached, the
  buffer stays `Empty`. The admitted edges {−1874999, 1874998} on both axes
  admit all nine halo producers and fail the resolver only on presence;
  {−1875000, 1874999} are refused with the expected axis and coordinate.
- **A2:** the corners are caller-trusted input, stated in the rustdoc of
  `SurfaceChunkInput` and in `INVARIANTS.md`. The replay checks the GEN-05.3
  producer corners against `groups.tsv` for all 70 groups; a test drives foreign
  corners and shows the result is still published but differs (minimum surface,
  final digest).
- **A3 (C3):** through the real write path with synthetic rule trees on genuine
  chunks: eroded raises (`biome/eroded_badlands`, `seam/eroded_pillar`: 512
  steep reads, 102 of a raised earlier column, 6 of the raised current column)
  and air-write rescans (`biome/plains`: 256 reads, 480 earlier, 64 current).
  Every read equals an independent live model and differs from post-NOISE where
  counted. Mutant "post-NOISE heights" is killed by both tests.
- **A4 (C5):** internal `cfg(test)` routines, labelled contract tests. Eroded
  loops from `min(floor, 319)` against the unclamped original (floors up to
  400, water abort, stone stop, `i4 > floor`); the frozen loop from
  `min(start, 320)` with 320 as `VOID_AIR`, minimum surface down to −200, the
  flat shape, snow then packed ice; `i4 = 320`; the heightmap rule. A start above
  320 is a defensive `Noise` refusal instead of a silent clamp (iterations
  above 320 would draw), so the equivalence never rests on `max_value`.
- **A5:** the alphabet is derived at bind time (GEN-05.4 material states through
  the material producer, 24 template states, band states, extension states) and
  is 35 for overworld, large_biomes and amplified; it contains all 32 observed
  states plus `calcite`, `red_sandstone`, `raw_copper_block`.
  `UnclassifiedState` is reached by a `cfg(test)` alphabet removal and by a
  material carrying `oak_log`.
- **A6:** the fixture manifest SHA-256 `98fcf62a…` and all seven file digests
  are checked before any use; states are identified by name through the
  registry (`compute_state_id` = raw id) and classified by name; no flag column
  is read.
- **RR-H1:** `large_biomes`/seed 1 rules against an overworld/seed 0 resolver
  yield `ResolverMismatch::Seed`, both in the session and through
  `build_chunk`. Mutant R01 is killed.
- **RR-H2:** on every one of the 2,261,637 replayed calls the session's
  `work_used` delta equals an independent model of the evaluator that charges
  the resolver iff a `biome` node or the `temperature` singleton is evaluated
  (consulted in 405,539 calls, not in 1,856,098; maximum 72 units per call).
  Mutant R06 (eager memo) is killed.

## Original replay (C1, C2, C4)

Expectations are mechanical projections of genuine Paper outcomes; policy and
contract tests are separate and never read original rows.

| Check | Result |
| --- | --- |
| Classes from the fixture policy table | 70 `full`, 1 `refused:CentreOutsideDomain`, 1 `refused:InconsistentResolver(Source)` |
| C2 producers per `full` group | GEN-05.4 material = `s0_columns_rle`, fluid flags = NOISE post-processing, S0 heights, GEN-05.3 corners, GEN-05.5 array digests: 70/70 |
| C1 per-column trace (9 tokens: `height_plus_1`, eroded writes, `first_call_height_plus_1`, surface depth, call runs, outcomes, steep, minimum surface, frozen writes) + column biome | 17,920/17,920 columns, 2,261,637 calls |
| Final digest, `WORLD_SURFACE_WG`, fluid flag set = final post-processing positions | 70/70 |
| Writes | eroded 139, rule 1,195,094, frozen 9,886; 838 steep tests, each equal to the independent live view |
| Refused groups | only the typed refusal: `Rules(InconsistentResolver(Source))` with a genuine fixed-plains resolver and an `Empty` buffer; A1 for the centre domain |
| C4 bits (2570 noise rows of `full` groups) | six noises Paper basis = Paper 15,420/15,420, Vanilla basis = Vanilla 15,420/15,420, Paper basis = Vanilla 0 (control); int draws and four f32-rounded doubles 2570/2570, unrounded = Vanilla 2570/2570; melt 2570/2570 (2269 true) |

## Work, capacities, allocation (C6)

- `W_col = 1 + max(1155, 1159) + 1538 = 2698`, `R = 198,144 + 256 · 2698 = 888,832`.
  **Deviation from the contract figures (2697 / 888,576), for Root:** the
  frozen bound gains one loop step because A4 makes the loop start at
  `min(start, 320)` — up to 385 iterations instead of 384. Draws stay ≤ 384
  (`(int) d4 < start ≤ 320`, so the 320 step never draws) and writes ≤ 384,
  hence 1159 instead of 1158. Observed maximum on the fixture: 517 per column
  (the review's figure), 270,785 per chunk (C0 counted as actually used),
  session at most 784,542 of 45,908,736.
- `build_chunk` refuses at `R − 1` (counts `(R, R − 1)`), publishes at `R` and
  `R + 1`. Budget domains 0/1/max/max+1. Bind census 8320 + 35 = 8355: refused
  at 8354, admitted at 8355/8356.
- Retained bytes (pinned): binding 10,816 (576 value + 10,240 noises; cap
  16 KiB), buffer 406,592 (1,088 + 393,216 + 12,288; cap 448 KiB); `NativeLimit`
  at N−1, admitted at N and N+1, alphabet cap 34/35/36.
- Allocation (`measure_allocations.py`, GEN-05.7 probe preloaded, fresh process):

| Phase | Debug (binary `203ed797…`) | Release (binary `b25ebd62…`) |
| --- | --- | --- |
| CONTROL (positive, 4096 bytes) | 1 call, 4096 bytes | 1 call, 4096 bytes |
| BUILD: bind + buffer (counted, not claimed free) | 3150 calls, 984,339 bytes | 3150 calls, 984,339 bytes |
| DRIVE: 70 published + 70 refused `build_chunk` | **0 calls, 0 bytes, 0 frees** | **0 calls, 0 bytes, 0 frees** |

  The probe's stack-write observation for DRIVE is 24,656 bytes (Debug) and
  16,384 bytes (Release). Linux/glibc observations for these exact binaries;
  no RSS, TPS or latency claim.

## Mutants

`.codex-tmp/gen05_10-runtime/mutants/run_mutants.py` (`c50a4246…`): each
mutant is one substitution in a separate copy whose World equals the candidate
(never the work tree), release build with its own target, the column suite,
source restored by hash. Results `mutants/results.json` (`31f4a133…`); an
identical earlier run on the pre-candidate suite is kept in `pre-candidate/`.

| Mutant | Result |
| --- | --- |
| A3 — `evaluate` receives the post-NOISE heights | killed by both C3 tests |
| R01 — resolver settings before seed | killed by the RR-H1 test |
| R06 — eager biome memo on every `evaluate` | killed by the replay (RR-H2 work model), C3, C5 `i4 = 320` and A2 tests |
| D3 — refusal does not clear the publication | killed by three refusal tests |
| air write at the top without rescan | killed (replay, C3 air, heightmap rule) |
| fluid flag of fluid writes omitted | killed (replay, heightmap rule) |
| eroded water abort removed | killed (replay, C5 eroded) |
| frozen start above 320 clamped silently | killed (C5 frozen) |
| frozen extra draw (short-circuit broken) | killed (replay, C5 frozen) |
| snow counter only for in-range writes | survives — **equivalent**: a hit above 319 needs y = 320, which reads `VOID_AIR` and cannot draw because `(int) d4 < start ≤ 320` |

## Final checks (unchanged candidate)

Candidate `candidate-01.source.json` (SHA-256
`cae56ed8f767e448bd33331d8dda692001529241705c75f85d285211f3991b93`, 324 World files: 33 added, 6 modified —
`README.md`, `INVARIANTS.md`, `src/surface_rules.rs`, `src/density.rs`,
`src/density/chunk.rs`, `src/density/chunk/material.rs` — 0 removed, 0
outside World) and its readonly copy `candidate-01-src/` were written before
the checks. All checks ran unchanged on the work copy with the fresh target
`final-target`, `--jobs 2 --locked`, cargo/rustc 1.99.0
(`final/summary.log` `2f2121d2…`):

| Check | Command | Result |
| --- | --- | --- |
| World Debug | `cargo test --locked --jobs 2 -p ferrumc-world --no-fail-fast -- --test-threads=2` | 9 suites, 383 passed, 0 failed, 4 ignored (allocation probes); 291 s |
| World Release | same with `--release` | identical counts; 153 s |
| Clippy | `cargo clippy --locked --jobs 2 --workspace --all-targets -- -D warnings` | pass |
| fmt | `cargo fmt --all --check` | pass |
| rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc --locked --jobs 2 -p ferrumc-world --no-deps` | pass |
| Workspace build | `cargo build --locked --jobs 2 --workspace` | pass |
| World consumers (7 direct + 2 transitive per `cargo metadata`) | `cargo test --locked --jobs 2 --no-fail-fast -p ferrumc-storage -p ferrumc-sim -p ferrumc-anvil -p ferrumc-session -p ferrumc-testkit -p ferrumc-bench -p ferrumc-app -p ferrumc-plugin-block-rules -p ferrumc-plugin-region-guard` | 94 suites, 1489 passed, 0 failed, 1 ignored; `multiplayer_visibility` passed, no rerun |
| Allocation | `measure_allocations.py` on the Debug and Release unit-test binaries | as above |

The column driver contributes 23 passed tests and 1 ignored probe per profile.

Audits before, after and at freeze (`final/audit-{before,after,freeze}.json`,
before `46d2c3e8…`, after `5975e913…`) agree in every check: the archive still
equals the base (1393 files); the 1102 outside-World work files equal the base
(the app/plugin tests left only three known `.codex-tmp/p57-…`/`p58-…` plugin
libraries in the copy root); the work World equals the candidate and its
readonly copy; the shared-tree World equalled the base; the frozen GEN-05.10
evidence (321 entries) and the contract, review, authorization and work order
are unchanged.

After the green checks and the mutant run: `source.freeze.json` (SHA-256
`3b457b812cd23f9f5bb7c172e3e57103d191b3f6c1f4ad0ad6b82f541d6999e2`, 324 files, equal to the candidate) and
`evidence.freeze.json` (`af30fb8f63562bbac0b5f7834782cf650664471e33729cc5c1399a5bd75b728e`, 60 files; work and
mutant copies, target directories and the development measurement excluded).
Only then was the World crate installed into the shared tree
(`install-report.json` `3b88e685…`): 39 files written (33 added, 6 modified),
the shared World now equals the source freeze; the four untracked
`__pycache__` files stayed untouched and nothing else in the shared tree was
written apart from this document.

Forbidden-claims scan (`markdown_blocks`/`rustdoc_blocks` + `scan` of the
archive's `scripts/check-forbidden-claims.py`, with and without
`force_plugin`; `forbidden-claims.json` `512cd7ca…`): this document, the
column reference README and the rustdoc of every `surface_rules` file have no
finding; the World `README.md`/`INVARIANTS.md` findings are the pre-existing
ones of the base (identical block digests), none in the new sections.

## Non-claims and open points

- No TPS, RSS, latency or full-server claims; no Survival, Java plugin or
  natural-world activation claims. Nothing installs the buffer into a live
  `Chunk`.
- `OCEAN_FLOOR_WG` and the post-processing order are not published (D5); a
  `blocksMotion` and a general fluid table remain prerequisites of carvers and
  features (H6). Later post-processing consumers must rebuild the order (H5).
- `Temperature` is unreachable for admitted inputs (frozen column biomes are
  always in the temperature table); its mapping is covered by code only.
- Source-based, no genuine case: `i4 = 320`, a steep test reading a height
  changed in the same chunk, eroded floor > 319, frozen minimum < −64 (now
  covered by the C3/C5 contract tests).
- Root decision requested: accept `W_col = 2698` / `R = 888,832` (A4
  consequence) instead of the contract figures 2697 / 888,576.
