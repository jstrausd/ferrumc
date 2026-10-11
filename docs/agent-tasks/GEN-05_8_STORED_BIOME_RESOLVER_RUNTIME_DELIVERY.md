# GEN-05.8 — stored biome resolver runtime delivery

Runtime delivery, 11.10.2026. This is a World-only implementation of the
approved [contract](GEN-05_8_STORED_BIOME_RESOLVER.md) (SHA-256 `75ffda46`).
It follows the [review](GEN-05_8_STORED_BIOME_RESOLVER_REVIEW.md) (SHA-256
`c85ff297`) and the [runtime authorization](GEN-05_8_STORED_BIOME_RESOLVER_RUNTIME_TASK.md)
(SHA-256 `5e4e4958`) with Root decisions A1–A5.

Written: `server/crates/ferrumc-world/**`, `.codex-tmp/gen05_8-runtime/**` and
this document. No Git operation, no other agent, dependency, lockfile,
generated file or other crate was touched.

The contract, its review, the frozen oracle `.codex-tmp/gen05_8/**` (freeze
manifest SHA-256 `d838de6c3af6d512d9b47741d8ed2be8b476337885bb0487d1a51435ea296cee`,
all 211 entries re-verified) and the historical GEN-05.5/.6/.7 artefacts are
unchanged. This delivery does not execute Java again.

## Basis and isolation

- Root's readonly Git archive of `main` `75f9824a8190c5ce21ef29bacb537a9f92794327`
  (1277 files, `base.freeze.json` SHA-256
  `cc5d57dd8ad9c7101501de388abea4a8ba3ea004a683dc396f04cc67d836ff54`) was
  byte-verified and copied privately to `.codex-tmp/gen05_8-runtime/work`.
- Only the whole World crate was changed there. Every check ran on that copy;
  the uncommitted Storage WIP in the shared tree was never built or read.
- Before installation, the shared tree's `ferrumc-world` equalled the base
  byte for byte. The only exceptions were four pre-existing untracked
  `__pycache__/*.pyc` interpreter caches, which were left as they were.

## Implemented contract

New public module `stored_biome`:

| Item | Content |
| --- | --- |
| Types | `StoredBiomeIdentity`, `StoredBiomeBuildBudget` (1..=1048576), `StoredBiomeQuartPos` (`new`, `from_zoom`), `StoredBiomeNeighborhood<'a>`, `SurfaceBiomeResolver<'a>`, `StoredBiomeError` with `kind/chunk/quart/block/input_index/limit/requested/maximum`, `StoredBiomeErrorKind`, `StoredBiomeLimit`, `StoredBiomeAxis` |
| Caps | 9 arrays, 2048 values per array, 18432 borrowed values, 512 retained bytes, 4096 bytes of typed construction scratch |

All fields are private. No `unwrap`/`expect` appears outside tests, and
`forbid(unsafe_code)` is unchanged.

The neighborhood borrows only sealed GEN-05.5 `ChunkBiomeSamples` in
`[Option<&'a _>; 9]`. It derives the zoom seed exactly once with
`BiomeZoomSeed::from_world_seed(identity.seed())`, after the work admission,
and retains nothing else.

Construction refuses in this order:

1. chunk count;
2. centre domain;
3. retained and scratch ceilings;
4. per array, in caller order: window/key domain, duplicate, seed, settings,
   source, geometry, length (`ValuesPerChunk` cap, then `IncompleteArray`),
   borrowed total, history shape (O(1));
5. the whole work requirement `2 + 10·N + values` (+9 for surface), refused
   before any value scan or hash;
6. the value scan: Fixed equality and the final MultiNoise slot;
7. for surface, the halo.

Lookup follows the contract:

- select the chunk `(qX >> 2, qZ >> 2)`;
- compute the i64 window slot `(dx+1)·3+(dz+1)`;
- refuse with `OutsideNeighborhood`, then `CoordinateDomain`, then `MissingChunk`;
- clamp only Y to `[minY>>2, minQY+(height>>2)−1]`;
- read the index `((qY−minQY)·4+(qX&3))·4+(qZ&3)`.

`InvalidStoredIndex` is defensive and is only reachable through a private
corrupted control. There is no fallback biome.

### Root decisions

- **A1.** `admit_chunk` became `pub(crate)` in `density/chunk.rs`. Since
  `density::chunk` is a private module, a one-line `pub(crate) use
  chunk::admit_chunk;` was also needed in `density.rs`; no other `.3` change
  was made. The module is registered in `lib.rs`. Boundary tests cover
  −1_875_000/1_874_999 and −1_875_001/1_875_000 on X and Z, plus the i32
  extremes.
- **A2.** `tests/reference/stored-biome/stored_biome_fixture.py` has two
  commands (details under "Original evidence and fixture"):
  - `generate` mechanically projects the freeze-pinned `final-normal`
    projection;
  - `validate` regenerates into a fresh directory, compares bytes, and
    re-expands every committed row back to the frozen source tokens.
  The 5568 controlled hook rows are policy rows and the 32 exception rows are
  evidence only.
- **A3 (substitute, reported to Root before the checks).** The accepted
  GEN-05.6 projection has `vanilla_paper_differences` = 0 in all five record
  kinds (D/F/G/H/L), so no five divergence positions exist. Vanilla
  `(b/1024−0.5)·0.9` and Paper `(b−512)·(0.9/1024)` are the same exact product
  with one rounding; all 1024 buckets are bit-identical. No output test can
  therefore discriminate the modes. Instead, a `cfg(test)` thread-local
  observation point in the single composition `Core::zoomed` records the exact
  `BiomeZoom` handed to `select_quart`. Tests check, for both semantics and for
  every genuine G row on the public neighborhood and surface paths, that this
  `BiomeZoom` carries the requested semantics and the derived seed. Hard-wired
  Vanilla and hard-wired Paper mutants both fail.
- **A4.** `StoredBiomeErrorKind::BlockCoordinateDomain { axis:
  StoredBiomeAxis, coordinate: i32 }`, documented as FerrumC policy. X is
  checked before Z, both before surface-column and lookup checks. `block()` is
  set and `chunk()` stays empty.
- **A5.** The fixture manifest, reference README, INVARIANTS and this document
  state that Vanilla ran with `-Dmax.bg.threads=1` on one background worker.
  Shared forward/reverse Vanilla histories are therefore single-worker
  sequences.

### Interpretations, stated openly

- **Halo order.** "All nine required keys in slot order: their .3 domain
  first, then presence" is implemented as two passes: every slot's domain,
  then every slot's presence. An edge centre therefore always refuses with
  `CoordinateDomain`, regardless of which arrays were supplied.
- **Hook rows under the block policy.** 1008 of the 5568 hook rows (G,
  `fulli32`) have block X/Z outside ±30M. For these, the public block query
  returns the A4 kind first. In addition, every hook row checks that the
  original delegated quart is refused by the kernel with `OutsideNeighborhood`
  for the original requested chunk.
- **Error size.** `StoredBiomeError` is 88 bytes, not the design's 80,
  because the A4 kind carries data.

## Original evidence and fixture

Fixture `tests/fixtures/stored-biome-original-1.21.8`:

| File | SHA-256 |
| --- | --- |
| `queries.tsv` (8.4 MB) | `7e326103…` |
| `arrays.tsv` | `e496dfcc…` |
| `histories.tsv` | `45a423cc…` |
| `manifest.json` | `79732818…` |

The selection rule is every row. Vanilla and Paper rows are stored once
because their query, array and history projections are byte-identical in
normal and `-O` mode; this is checked against the pins. `validate` passes in
normal and Python `-O` mode with byte-identical output. No numerical
expectation formula is involved.

Mechanical original equality, Debug and Release:

- **Axis rows:** all 131392 axis value rows (B 3328, G 79872, Q 48192) under
  both zoom semantics, through the shared checked kernel with private controls.
  Each control is built from the original listing order, which the generator
  proves is the (quart Y, X, Z) sort of the original quarts. Every row compares:
  - the original delegated quart (G via the .6 selector, B via `>>2`);
  - the original requested chunk;
  - the resource;
  - the zoom bits;
  - all 7 geometries, including min/top clamps and Y extremes.
- **Real arrays:** all 180 actual GEN-05.5 arrays (254592 values) produced by
  real `RouterTemplate` → `fresh_paper_noise_chunk` → `BiomeSourceSession`
  equal the original whole arrays.
  - Initial and final leaf ordinals of all 180 histories match.
  - Cold groups use separate cold sessions.
  - The four forward/reverse groups continue one session; 32 continuing
    arrays were checked.
- **Real neighborhoods:** all 20 genuine nine-chunk neighborhoods answer every
  G row (3840 each, both semantics, neighborhood and surface) and every Q row
  (360 each). Each is built in three borrowing orders (original, reversed,
  rotated) with copied values, for 482400 row evaluations in total. Delegated
  quart, requested chunk and resource match.
- **Policy rows:** 4560 rows end in `OutsideNeighborhood` and 1008 in the
  block-domain refusal (both modes). All 30 accessor exception geometries are
  refused by `BiomeQuartGeometry`.

Policy and malformed controls are tested separately from original equality:

- budget 0/1/1048576/1048577;
- counts 0/1/8/9/10;
- every missing halo slot;
- duplicates first, last and multiple;
- each inconsistent component, with both controls and real arrays:
  - seed: seed 0 vs −1;
  - settings: normal vs large_biomes;
  - source: MultiNoise vs single_biome_surface Fixed;
  - geometry: −64/384 vs −80/416;
- wrong/partial/extra/oversized lengths (0, 63, 65, 2047, 2049);
- invalid history: missing final, ordinal 5/7593, ordinal/biome mismatch,
  initial, final vs last slot;
- Fixed/End with history, Fixed content;
- geometry: 1/32/33 sections, checked min/top, signed chunk extremes;
- sparse vs surface centre domains, quart and block refusal order, window
  edges;
- ±30_000_000/±30_000_001 and i32 extremes;
- surface column and every signed32 Y;
- work bounds:
  - 75/76/77 on a 64-value control;
  - 18523/18524/18525 and 18532/18533/18534 on the real
    single_biome_surface −64/512 arrays (9 × 2048 = 18432 values);
  - refusal before the value scan;
- native caps at N−1/N/N+1, including a retained-ceiling fault through the
  real admission;
- mutation-free failed publication;
- defensive `InvalidStoredIndex`.

Six `compile_fail` doctests cover:
- `BiomeZoomSeed` in place of `WorldSeed`;
- `ClimateQuartPos` and `BiomeQuartPos` in place of `StoredBiomeQuartPos`;
- a detached `'static` neighborhood;
- an `i32` wire ID;
- `BlockPos` as a quart.

Each was extracted separately: it fails with exactly one E0308 or E0515
error, and its corrected counterpart compiles (`compile_fail_check.py`,
`compile-fail-check/report.json`).

The mutation check `mutants/results.json` (SHA-256 `99535de1…`) ran 11
temporary source mutants. All 11 make the tests fail:
- Y/Z/X index order;
- missing Y clamp;
- swapped slot;
- truncating chunk division;
- hard-wired Vanilla or Paper;
- unshifted block Y;
- Fixed scan disabled;
- work off-by-one;
- open block policy;
- obfuscated zoom seed.

The source was restored by hash after the mutation run.

## Native layout and allocation (observations)

Debug and Release on x86_64, Rust 1.99.0 and glibc 2.44 report identical
`size_of` values (`final-layout/{debug,release}.log`):

| Type or quantity | Bytes |
| --- | --- |
| identity | 24 |
| neighborhood | 128 |
| surface | 136 |
| error | 88 |
| quart | 12 |
| budget | 4 |
| typed construction scratch census | 1472 (≤ 4096) |
| borrowed: 9 array objects | 504 |
| borrowed: max value capacity | 18432 |

Borrowed bytes stay separate from retained bytes and are not RSS.

The external probe is the unchanged GEN-05.7 `allocator_probe.c`. It ran the
exact final unit-test binaries in fresh processes:
- Debug `1b1fde15…`;
- Release `345cc9e9…`.

Measured operations: 67 admitted and 60 refused builds (including the
process's first build), 100 copy rounds and 532576 corpus queries.

BUILD, CLONE and QUERY show 0 calls, 0 bytes and 0 frees. CONTROL shows 1
call, 4096 bytes and 1 free.

Observed stack-write high water:
- Debug BUILD: 20652 bytes;
- every other phase: below the protected 16384-byte marker margin.

These stack writes are reported separately from the typed scratch census.
The census is not a stack bound.

## Final checks (unchanged candidate)

The candidate manifest `candidate-01.source.json` (SHA-256
`24a90f864a3d2b3c780d5b8b17e398f06b6948cad88e79869d2b5a705bf26d95`, 245 World
files) was reported to Root before the checks. The checks ran in a fresh
target directory and wrote `final/checks.json` (SHA-256 `50260a22…`).

| Check | Command | Result |
| --- | --- | --- |
| Debug | `cargo test -p ferrumc-world --jobs 2 --locked -- --test-threads=2` | 320 passed, 0 failed, 2 ignored (two probe harnesses) in 9 suites: lib 264 + 1 ignored, integration 46 + 1 ignored, doc 10 |
| Release | same with `--release` | identical counts |
| Clippy | `cargo clippy -p ferrumc-world --all-targets --jobs 2 --locked -- -D warnings` (workspace pedantic) | pass |
| fmt | `cargo fmt --all --check` | pass |
| rustdoc | `RUSTDOCFLAGS="-D warnings" cargo doc -p ferrumc-world --no-deps --jobs 2 --locked` | pass |

Of these, `stored_biome` contributes 29 passed unit tests, 1 ignored probe
and 6 compile-fail doctests.

Audits before and after were identical:
- the archive still has 1277 files, byte-equal;
- 1044 outside-World files are byte-equal to the base;
- the World matches the candidate;
- the shared-tree World still equalled the base.

Source and evidence freezes were written only after these green checks
(`source.freeze.json`, `evidence.freeze.json`). Their hashes go to Root
separately. Afterwards only the World crate was installed into the shared
tree, and the installed bytes were verified against the candidate.

## Non-claims and open points

- No TPS, RSS, latency or full-server claims. No Survival, Java plugin or
  natural-world activation claims.
- No WorldGenRegion, ChunkMap, SurfaceSystem, carver or server execution. The
  3×3 halo remains source review plus differential evidence.
- No surface-rule compilation or SurfaceSystem driver; those are the next
  World tickets.
- No genuine fallback producer for missing chunks; the typed refusal is
  FerrumC policy.
- The A3 discrimination is observational: the output corpus cannot
  discriminate the modes.
- Vanilla multi-worker scheduling is not claimed.
- End arrays are not checked against the five End IDs (H5, optional).
- The allocation and stack figures apply only to the named binaries and
  toolchain.
