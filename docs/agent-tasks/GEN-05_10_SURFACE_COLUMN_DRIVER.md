# GEN-05.10 — overworld SurfaceSystem column driver: concrete contract and genuine original oracle

Preparation delivery, 12.10.2026, for the [work order](GEN-05_10_SURFACE_COLUMN_DRIVER_TASK.md).
The contract and oracle review belongs to Root; this document does not authorize a runtime. Only
this document and `.codex-tmp/gen05_10/**` were written. No Git work, no production edit, no new
dependency, no other agent and no change to a historical GEN-05.x artefact. Design probes ran only
in a private archive copy with its own target directory (`--jobs 2`).

Labels used throughout: **source review** (read, not executed: §2.1, `sources/`),
**differential evidence** (genuine original execution: §2.2, `final-*`), **contract model check
(derived)** (a second-route replay of this contract's text against genuine writes: §2.3) and
**design probe** (FerrumC producers against genuine cells in a private copy: §2.4). A **FerrumC
policy** (marked D*n*, or inherited P*n* from GEN-05.9) is never presented as original behaviour.

## 1. Basis, scope and consumed producers

Root's readonly archive of main `d97fcc5c49148d1f0713022a9a603119da1d44ef`
(`.codex-tmp/gen05_10-root-base/archive`, 1386 files, `base.freeze.json` SHA-256
`c6266dcb7e6be4608dde6f74beb7422079730fd9284117f5e1dc77d0c443321d`) contains the integrated GEN-05.9
runtime. `oracle/prepare.py` re-hashed every archive entry before pinning; code was read only from
that archive. The DUR-03.3 and SIM-03.4c work in the shared tree was ignored.

Original artefacts are the ones the accepted GEN-05.8 oracle pins (`gen05_8/oracle/input-pins.json`
`bffbed18…50d8`, freeze `d838de6c…6cee`), re-verified byte for byte: Vanilla 1.21.8 inner JAR
`2bee159e…0250` with 44 libraries, Paper build 60 (commit `29c8822d90899c89d2689338e81a98f690bcba12`)
inner JAR `c64fb7db…1507` with 119 libraries, the complete JDK 21.0.12.1 (454 files) and the official
`server-mappings.txt`. The accepted GEN-05.9 freeze (`round3/delivery/freeze-manifest.json`
`9f09cbd5…0e11`) and its helper are pinned as consumed references. No new original artefact was
downloaded. `oracle/input-pins.json` holds 2123 inputs, SHA-256
`51b52a58f80b7da551c84d3516618da47c57bccaf1698fb819d21dbb64cd4f94`.

Scope: the original `SurfaceSystem.buildSurface` column driver of the overworld SURFACE step —
column loop, the stone-depth/water-height derivation, the column biome, the eroded-badlands prepass,
the frozen-ocean postpass, every block write with its heightmap and post-processing effect — driving
the accepted GEN-05.9 rule evaluator. Out of scope (work order): carvers, features, structures,
lighting, natural-world persistence or activation, Nether and End, Survival and Java plugins.

Consumed accepted producers (all unchanged; the design probe of §2.4 proves each one delivers the
genuine driver input for every admitted oracle chunk):

| Producer | Use in the driver |
| --- | --- |
| GEN-05.4 `NoiseChunkSession::sample_chunk_materials` → `ChunkMaterialSamples` | the post-NOISE chunk (states, ascending Y/X/Z) and the NOISE post-processing fluid flags |
| GEN-05.3 `NoiseChunkSession::preliminary_surface_level` | the four `PreliminarySurfaceCorners` of the GEN-05.9 session |
| GEN-05.5 `BiomeSourceSession::sample_chunk_biomes` | the nine stored arrays (cold history, row-major) |
| GEN-05.8 `SurfaceBiomeResolver::biome_at_block` | the column biome at `(x, height + 1, z)` and every per-block biome |
| GEN-05.7 `BuiltinBiomeTemperatures::sample(..).should_melt_iceberg_slightly()` | the frozen-ocean melt predicate |
| GEN-05.9 `SeededSurfaceRules`, `SurfaceRuleSession` | rule evaluation, surface depth, minimum surface, positional root |
| GEN-04 `random`, `noise::NormalNoise` + crate-private `paper_build60_basis` | six extension noises and the extension draws |
| Registry `NoiseResourceId::parameters`, `block_state::{block_metadata, compute_state_id}` | noise parameters, the closed state alphabet |

## 2. Original facts

### 2.1 Source review (read, not executed)

`sources/collect.py` copied whole Base (paperweight mache = Vanilla 1.21.8 decompiled) and Applied
(Paper at `29c8822d`) files of the closure, every per-file source patch, the two feature patches that
touch the closure (`0004-Anti-Xray`, `0016-Moonrise-optimisation-patches`), the Paper-only step
caller and the overworld resource patch: 92 files, byte-pinned in `sources/source-map.json`.
Line numbers below refer to `sources/copies/applied/…`.

1. **Unpatched.** `SurfaceSystem`, `SurfaceRules`, `Heightmap`, `BlockColumn`, `Noises`, `NoiseChunk`,
   `RandomState`, the random sources and noises, `Mth`, `LevelHeightAccessor` are byte-identical in
   Base and Applied, and no feature patch touches them.
2. **Paper changes inside the closure are outside the SURFACE write semantics.**
   - `ProtoChunk`: a `getBlockState(int,int,int)` overload (same section lookup) and Moonrise drops
     the `skyLightSources` update; light work runs only at persisted status ≥ `INITIALIZE_LIGHT`.
   - `ChunkAccess`: CraftBukkit/Moonrise/Anti-Xray fields; `getHeight` (260-272) unchanged.
   - `NoiseBasedChunkGenerator`: `WorldGenerationContext(this, level, level.getMinecraftWorld())`
     (flat-bedrock carrier, GEN-05.9 P2) and synchronous `Runnable::run` executors.
   - `WorldGenRegion`: if-loaded helpers, far-write warning and block-entity clearing inside
     `WorldGenRegion.setBlock`; `buildSurface` never calls it (it writes the chunk directly).
   - `Biome`, `BiomeManager`, `LevelReader`, `BlockBehaviour`: Moonrise optimisations (the removed
     Vanilla temperature cache is known from GEN-05.7, the zoom from GEN-05.6/.8) and Paper's
     explosion `isDestroyable` check; none changes a value the SURFACE step reads.
   - `ChunkStatusTasks`, `ChunkPyramid`, `ChunkStep`: structure flag, entity loading, Moonrise. The
     Moonrise scheduler still applies `ChunkPyramid.GENERATION_PYRAMID.getStepTo(SURFACE)`
     (`ChunkUpgradeGenericStatusTask.java:99`), i.e. `ChunkStatusTasks.generateSurface`.
3. **SURFACE step.** `ChunkPyramid` (24-29): SURFACE requires `STRUCTURE_STARTS` radius 8 and `BIOMES`
   radius 1, block-state write radius 0. `generateSurface` (110-118) builds a `WorldGenRegion` and
   calls `NoiseBasedChunkGenerator.buildSurface(region, structureManager, randomState, chunk)` (219):
   unless `debugVoidTerrain`, it builds the `WorldGenerationContext` and calls the overload (235) with
   `region.getBiomeManager()`, the biome registry and `Blender.of(region)`. The overload takes the
   chunk's retained NOISE-step `NoiseChunk` (`getOrCreateNoiseChunk`) and calls
   `randomState.surfaceSystem().buildSurface(random, biomeManager, biomes,
   settings.useLegacyRandomSource(), context, chunk, noiseChunk, settings.surfaceRule())`. The region's
   `BiomeManager` resolves `LevelReader.getNoiseBiome` through `getChunk(.., BIOMES, false)` of the
   3×3 neighbourhood (GEN-05.8).
4. **Column loop** (`SurfaceSystem.java` 66-163). Columns: `i` (local X) outer, `i1` (local Z) inner.
   Per column: `i4 = chunk.getHeight(WORLD_SURFACE_WG, i, i1) + 1` (110); column biome
   `biomeManager.getBiome((x, useLegacy ? 0 : i4, z))` (112); eroded prepass iff that biome is
   `eroded_badlands` (113-115); `i5 = getHeight(WORLD_SURFACE_WG) + 1` re-read (117);
   `context.updateXZ(x, z)` (118); `i6 = 0, i7 = MIN, i8 = MAX` (119-121); `y` from `i5` down to
   `minY` (124): air → `i6 = 0, i7 = MIN`; fluid → `i7 = y + 1` if `i7 == MIN`; otherwise (stone-like)
   if `i8 >= y` the stone-below search reads `y-1` down to `minY-1` and sets `i8` to one above the
   first non-stone (134-144), then `i6++`, `updateY(i6, y - i8 + 1, i7, x, y, z)` (148) and, only if
   the block **is** the default block state, `tryApply` and on non-null `setBlock(y, state)`
   (149-154). After the loop the frozen postpass runs iff the column biome is `frozen_ocean` or
   `deep_frozen_ocean`, with `context.getMinSurfaceLevel()` and the stale `i4` (158-160).
5. **Writes** (`BlockColumn` 80-101): `setBlock` writes only inside the build height, through
   `ProtoChunk.setBlockState` (136-190): section write; no light below `INITIALIZE_LIGHT`; for every
   heightmap of `getPersistedStatus().heightmapsAfter()` — at SURFACE the persisted status is NOISE, so
   `WORLD_SURFACE_WG` and `OCEAN_FLOOR_WG` (`ChunkStatus` 17-26) — `Heightmap.update` (81); no block
   entity is created or removed. A written state with a non-empty fluid is then marked with
   `markPosForPostprocessing` (282-286: append the packed offset to the section's `ShortList`).
   `getBlock` outside the build height returns `VOID_AIR`.
6. **Heightmaps.** `getHeight` = `firstAvailable − 1` (271), i.e. the top Y whose state satisfies the
   predicate (`NOT_AIR` for `WORLD_SURFACE_WG`, `blocksMotion` for `OCEAN_FLOOR_WG`), or `minY − 1`.
   `update` raises on an opaque write at or above the top and, for a non-opaque write exactly at the
   top, rescans downward through the chunk (81-104); writes further below change nothing.
7. **Eroded prepass** (205-231) and **frozen postpass** (233-272): §6. Both use their own three
   `SurfaceSystem` noises (`badlands_surface/pillar/pillar_roof`, `iceberg_surface/pillar/pillar_roof`),
   the frozen postpass a fresh `noiseRandom.at(x, 0, z)` and `Biome.shouldMeltFrozenOceanIcebergSlightly`
   (`Biome` 162: `getTemperature((x, seaLevel, z), seaLevel) > 0.1F`).
8. **`SurfaceRules.Context`** (307-372): `updateXZ` recomputes the surface depth and invalidates every
   per-column cache; `updateY` sets the per-block inputs and a memoized biome supplier;
   `getMinSurfaceLevel` is cached per `updateXZ`. Steep reads `getHeight(WORLD_SURFACE_WG)` of the
   current chunk lazily at the column's first steep test (`SteepMaterialCondition` 393-420).

### 2.2 Differential evidence (genuine original execution)

The JDK-only helper `oracle/SurfaceColumnOracle.java` drives actual original objects by reflection
(Vanilla through the pinned official mappings, Paper through Mojang names) and verifies the
CodeSource of every original class, library and the freshly compiled helper inside the JVM. It
implements no column loop, extension, noise, random, temperature or zoom logic. One run has three JVM
stages:

1. **scan (Paper, selection only):** genuine `MultiNoiseBiomeSource.getNoiseBiome` lattice (seed 0,
   normal, 256-block ring order to radius 49152, y ∈ {64, 80, 100, 120, 150}), then genuine Paper
   generation plus an instrumented SURFACE run of each candidate chunk. A biome target is accepted
   at the first candidate with ≥ 32 columns of that genuine column biome. Twelve declared selection
   predicates are evaluated on the genuine instrumented outputs; `frozen_hole` and `rule_air_write`
   draw their candidates from a genuine `SurfaceSystem.getSurfaceDepth ≤ 0` spiral, as GEN-05.9 did.
   117 genuine evaluations, 0 failures. Selection uses original outputs; every expectation remains an
   outcome of stages 2 and 3.
2. **vanilla** and 3. **paper:** for each of 72 groups (16 fixed + 51 biome + 3 feature + 2 seam) **two
   independent genuine generations** (U and R) with the GEN-05.9 discipline: `WorldPreset` overworld
   generator, `RandomState.create`, nine `ProtoChunk`s with level height (−64, 384), cold RTree
   history, `createBiomes` row-major with a `StructureManager` over `WorldOptions(seed,
   generateStructures = false)`, persisted status BIOMES, genuine `fillFromNoise` on the centre
   (`Blender.empty`), persisted status NOISE. U and R must be equal after NOISE (states, both
   heightmaps, post-processing, block entities, nine stored arrays).
   - **U (uninstrumented):** genuine `NoiseBasedChunkGenerator.buildSurface(ChunkAccess,
     WorldGenerationContext, RandomState, StructureManager, BiomeManager, Registry, Blender)` — the
     callee of the `WorldGenRegion` overload — with a genuine `BiomeManager` over a `LevelReader`
     proxy that serves exactly the nine BIOMES chunks via `getChunk(.., BIOMES, false)`, and the Paper
     `WorldGenerationContext(generator, LevelHeightAccessor(−64, 384), Level carrier)` with
     `generateFlatBedrock = false` (the accepted GEN-05.9 carrier).
   - **R (recorded):** genuine `SurfaceSystem.buildSurface` with exactly the arguments the generator
     passes and a transparent `RuleSource` proxy that applies the genuine `settings.surfaceRule()` and
     forwards every `tryApply`. At a column's first `tryApply` (before its first rule write) it
     snapshots the column and the current `WORLD_SURFACE_WG`; at every call it reads the original
     context fields; at the column's first steep computation it reads the four heights; before the
     next column it reads the cached minimum surface.
   - U and R final chunks must be equal: the proxy is transparent and the callee arguments are the
     generator's.
   - **Diagnostics** (after both runs, side-effect free): the six extension noises at the exact call
     arguments, a fresh `noiseRandom.at(x,0,z)` with `nextInt(4)`, `nextInt(10)` and four
     `nextDouble`, `shouldMeltFrozenOceanIcebergSlightly` on the column biome, the direct
     `BiomeManager.getBiome((x, height + 1, z))`, the four genuine preliminary corners and the
     positional factory seeds.

Which original code produced every recorded column input:

| Input | Original producer | Recorded |
| --- | --- | --- |
| post-NOISE chunk, both WG heightmaps, NOISE post-processing | `NoiseBasedChunkGenerator.fillFromNoise/doFill` + `Heightmap.update` + `markPosForPostprocessing` | full snapshot of U and R (`chunks.tsv` S0) |
| `i4` | `chunk.getHeight(WORLD_SURFACE_WG)` at column start = post-NOISE height (no earlier column writes this column) | S0 height; checked equal in both generations |
| column biome | `BiomeManager.getBiome((x, i4, z))` over the genuine 3×3 | direct genuine query |
| eroded writes | `erodedBadlandsExtension` | genuine column difference S0 → first `tryApply` |
| `i5` | re-read `getHeight` after the prepass | genuine `getHeight` at the first `tryApply` |
| surface depth | `Context.updateXZ` → `getSurfaceDepth` | genuine context field |
| per-call `y`, stone above/below, water | `buildSurface` loop → `Context.updateY` | genuine context fields at each `tryApply` |
| outcome | genuine `tryApply` | returned state |
| steep heights | `SteepMaterialCondition` reading `getHeight` | genuine `getHeight` at the computing call |
| minimum surface | `Context.getMinSurfaceLevel` (rule tree or frozen postpass) | genuine context field |
| frozen writes | `frozenOceanExtension` | genuine final column minus (first-call column + rule writes) |
| final chunk, heightmaps, post-processing, block entities | `ProtoChunk.setBlockState` etc. | full snapshot of U and R (`FINAL`) |

**Results** (fresh run `final-normal`; `final-optimized` (`python3 -O`) is byte-identical, §17):

| Item | Vanilla | Paper |
| --- | --- | --- |
| Groups / columns | 72 / 18432 | 72 / 18432 |
| genuine `buildSurface` executions (U + R) | 144 | 144 |
| genuine `tryApply` calls recorded (R) | 2,318,372 | 2,318,375 |
| original exceptions (generation, U, R) | 0 | 0 |
| U = R after NOISE and after SURFACE | 72 / 72 | 72 / 72 |
| neighbour chunks unchanged (highest filled section −1) | 72 / 72 | 72 / 72 |
| rule writes / of them fluid (water) / air | 1,228,551 / 121 / 4 | 1,228,568 / 121 / 4 |
| eroded writes (columns with writes) | 139 (53) | 139 (53) |
| frozen writes / onto water / snow block | 9886 / 5192 / 238 | 9886 / 5192 / 238 |
| post-processing marks after SURFACE / duplicates | 2974 / 0 | 2974 / 0 |
| block entities and pending block entities, before and after | 0 | 0 |
| steep tests | 838 | 838 |
| columns with ≥ 2 stone runs | 9483 | 9482 |
| calls whose water height comes from lava | 5085 | 5085 |
| distinct states | 32 | 32 |

Coverage (Paper, admitted groups): all 51 declared overworld surface biomes occur as genuine column
biomes (smallest: `frozen_river`, 48 columns), including the three badlands variants, frozen and deep
frozen oceans with icebergs, beaches, stony shore, rivers, every ocean depth/temperature, the peaks,
swamp and mangrove swamp; no cave biome occurs as a column biome. Negative coordinates (the 2×2 quad
around the origin, (−17, 9), (−1874998, −1874998)), chunk seams (the origin quad, an eroded-pillar
pair, an iceberg pair), seeds 0, −1, `i64::MIN`, `i64::MAX`, presets normal, large_biomes, amplified
and both originals. Selected features: eroded pillars, iceberg snow, iceberg writes onto water, melt
true and false, frozen-ocean hole, a rule write of air, swamp water. Declared but absent after the
declared search (honest gap, §15): a post-NOISE top at Y 319 (`i4 = 320`) even in amplified, and a
steep test reading a height changed earlier in the chunk.

**Vanilla vs Paper** (`final-normal/manifest.json`, `report.vanilla_vs_paper`):

- the six extension-noise values differ in **18432 / 18432** columns each, and every `nextDouble` of
  the extension draws differs (Paper rounds through f32, as GEN-05.9 found for the column draw);
  `nextInt` draws and the melt predicate are equal in every column;
- 62 final columns in 25 groups differ. First differing stage in `buildSurface` order: post-NOISE
  terrain 29, column biome 18, rule outcomes 15; **eroded prepass 0, frozen postpass 0, call inputs 0**;
  surface depth equal everywhere in these chunks.

Hence tests that presuppose a Vanilla/Paper difference are justified for the six noise bit patterns
and the `nextDouble` bits (direct comparisons), **not** for eroded or frozen write outcomes, column
loop inputs or `nextInt` draws. Paper cells are the only expectations (P1).

### 2.3 Contract model check (derived)

`oracle/project_originals.py` replays this contract's text (§3-§7) on every genuine column of both
originals, from genuine recorded inputs only, and compares with the genuine writes: the column-loop
call list (`y`, stone above/below, water) from the post-prepass column; the eroded prepass from the
three genuine badlands noise values and the S0 column; the frozen postpass from the three genuine
iceberg values, the genuine melt flag, the genuine minimum surface and a second-route Xoroshiro
(verified against the genuine fresh draws in every column); the steep view from the live heights;
both WG heightmaps as top-predicate functions of the final states; the final post-processing lists
as the NOISE lists plus fluid rule writes in write order. **0 mismatches** in both originals
(18432 columns, 2.3 M calls each). Branch census (Paper): eroded — `min ≤ 0` 244, height above floor
113, water abort 2, stone stop with writes 53; frozen — `min ≤ 1.8` 830, iceberg 304, flat
(`min1 ≤ 2` after melt) 1.

### 2.4 Design probe (private archive copy; design only)

`probe/run_probe.py` copies Root's archive, verifies it against `base.freeze.json`, adds exactly one
`#[cfg(test)]` module (`surface_column_design_probe.rs`) and its registration in
`ferrumc-world/src/lib.rs`, refuses any other difference and runs Debug (bits/alphabet part) and
Release (plus producers) with its own target directory. The shared part is byte-identical in both
profiles (`probe/final/receipt.json`).

| Check | Equal / compared |
| --- | --- |
| six extension noises, Paper basis (public derivation + `paper_build60_basis`) = genuine Paper | 18432 / 18432 each |
| six extension noises, Vanilla basis = genuine Vanilla | 18432 / 18432 each |
| Paper basis = genuine Vanilla (discriminating control) | 0 / 18432 each |
| `root.at(x,0,z)`: `next_int_bounded(4)`, `(10)`; four `f64::from(next_double() as f32)` = Paper | 18432 / 18432 |
| same draws unrounded = Vanilla | 18432 / 18432 |
| GEN-05.7 melt predicate on the genuine column biome = genuine, both originals | 18432 / 18432 |
| GEN-05.4 `sample_chunk_materials` states = genuine post-NOISE chunk (admitted groups) | 70 / 70 chunks |
| GEN-05.4 fluid flags = genuine NOISE post-processing positions | 70 / 70 |
| GEN-05.3 preliminary corners = genuine | 70 / 70 |
| GEN-05.5 arrays (cold, row-major) = genuine digests | 70 / 70 groups |
| GEN-05.8 resolver at `(x, height + 1, z)` = genuine column biome | 17920 / 17920 |
| state identity via `compute_state_id`; air (air/cave_air/void_air) and fluid (water/lava dense ranges) by registry metadata | 32 / 32 |

Design figures: the six noises have 32 `ImprovedNoise` (amplitudes 3, 4, 1, 3, 4, 1 octaves), bind
census 8320 (two stacks × (octave slots + 259 per `ImprovedNoise`)), 10240 retained bytes in the Paper
basis, `max_value` 5, 16/3, 10/3, 5, 16/3, 10/3.

## 3. Column loop and Y order (contract)

For one admitted centre chunk the driver performs, in this order (Java `int` arithmetic with
wrapping wherever the original computes in `int`):

1. Copy the GEN-05.4 material into the output buffer and derive the live `WORLD_SURFACE_WG` heights
   (top non-air Y per column or −65).
2. For `lx` in `0..16` (outer), `lz` in `0..16` (inner), block `x = 16·cx + lx`, `z = 16·cz + lz`:
   1. `i4 = height(lx, lz) + 1`.
   2. `biome = resolver.biome_at_block((x, i4, z))` (`useLegacyRandomSource` is `false` for every
      admitted setting; any `i4` in `[−64, 320]` is a valid resolver input).
   3. If `biome == eroded_badlands`: eroded prepass (§6.1).
   4. `i5 = height(lx, lz) + 1` (re-read).
   5. `session.begin_column(SurfaceColumnPos(chunk, lx, lz))` (GEN-05.9 `updateXZ`).
   6. `i6 = 0, i7 = None, i8 = +∞`; for `y` from `i5` down to `−64`, `s = state(y)` (`y ≥ 320` reads
      the void-air class):
      - air class → `i6 = 0, i7 = None`;
      - fluid class → `i7 = Some(y + 1)` if `i7` is `None`;
      - otherwise: if `i8 ≥ y`, search `t = y − 1, y − 2, …, −65` for the first `t` that is air or
        fluid (`−65` is void air) and set `i8 = t + 1`; then `i6 += 1`; if `s` is the default block
        state, `evaluate(SurfaceBlockInput::new(y, i6, y − i8 + 1, i7), &heights)` and on
        `State(t)` write `t` at `y` (§7).
   7. If `biome ∈ {frozen_ocean, deep_frozen_ocean}`: frozen postpass (§6.2) with
      `session.min_surface_level()` and the **stale `i4`** of step 1.

Inputs are never cached across columns. The height reads of steps 1 and 4 are live values after all
earlier writes of the chunk; step 7 deliberately reuses the step-1 value, as the original does. Writes
of the loop land only at the visited `y`, and the stone-below search reads only below it, so a write
never changes a later visit of the same column. Non-default stone-like blocks (ore veins: copper_ore,
raw_copper_block, granite, deepslate_iron_ore, raw_iron_block, tuff) advance `i6` without `evaluate`;
this equals the original, whose skipped `updateY` calls are result-neutral (GEN-05.9 §5).

## 4. Inputs: derivation, time and source

| Input | Derivation | When | Source in FerrumC |
| --- | --- | --- | --- |
| post-NOISE states | GEN-05.4 material, copied once | before column (0,0) | `ChunkMaterialSamples` (seed, settings, chunk, geometry checked, §9) |
| `WORLD_SURFACE_WG` | top non-air per column; maintained on every write with the original `update` rule | derived once, then live | driver buffer |
| `OCEAN_FLOOR_WG` | not maintained and not published (policy D5) | — | derivable later from final states with a `blocksMotion` table |
| `i4`, `i5` | live height + 1 before / after the prepass | §3 steps 1 and 4 | live heights |
| column biome | `biome_at_block((x, i4, z))` | step 2 | GEN-05.8 resolver |
| surface depth, hole | `begin_column` | step 5 | GEN-05.9 session |
| stone depth above `i6` | count of stone-like blocks since the last air (fluid does not reset) | per stone-like block | loop |
| stone depth below `y − i8 + 1` | distance to the first air/fluid below the current stone run (+1) | per stone-like block | loop search |
| water height `i7` | `None` after air; first fluid block above gives `y + 1` (lava counts as fluid) | per block | loop |
| minimum surface | `session.min_surface_level()` (cached per column; computed by the first `above_preliminary_surface` test or the frozen postpass) | rule tree or step 7 | GEN-05.9 session, GEN-05.3 corners |
| steep heights | the live heights at the column's first admitted steep test | lazily inside `evaluate` | `&heights` passed on every call |

All recorded genuine inputs lie inside the GEN-05.9 P4 domains: `y ∈ [−64, 319]`, stone depths
`1..=384` (`i8 ≥ −64`), water `None` or `Some(v)` with `v ≤ 320` (checked for all 2,261,637 calls of
admitted Paper groups by the fixture validator).

## 5. Steep and hole inputs

- **Hole** is `surface_depth ≤ 0` of the active column, computed at `begin_column` (GEN-05.9). 263
  genuine admitted columns have a non-positive surface depth.
- **Steep** reads `h(lx, min(lz+1,15))`, `h(lx, max(lz−1,0))`, `h(max(lx−1,0), lz)`,
  `h(min(lx+1,15), lz)` of the centre chunk only (the clamp reads the current column at edges; it
  never crosses a chunk seam) at the column's **first admitted steep test**. The driver passes its
  live `WorldSurfaceHeights` on **every** `evaluate` call. At that moment: columns earlier in the loop
  order hold their final heights (after their eroded, rule and frozen writes), the current column
  holds its height after the prepass and after every rule write above the current `y`, later columns
  hold post-NOISE heights.
- Evidence: all 838 genuine steep tests read exactly this live view (contract model check). In the
  oracle chunks no steep test read a height different from post-NOISE (the declared search for such a
  case found none: steep biomes never write air or raise heights), so the view's time dependence is
  source-based, as GEN-05.9 H6. Runtime condition C3 adds controlled tests.
- Updating the live view needs an in-place setter on `WorldSurfaceHeights`
  (`pub(super) fn set(&mut self, lx, lz, height)`); no public API change.

## 6. Extensions and writes outside the rule tree

All arithmetic is IEEE `f64` with Java `Math.abs/min/ceil` semantics; `Mth.floor(d) = (int) d − (d <
(int) d)`; `(int)` truncates toward zero.

### 6.1 Eroded-badlands prepass (iff column biome is `eroded_badlands`)

1. `min = min(|badlands_surface(x, 0, z) · 8.25|, badlands_pillar(0.2x, 0, 0.2z) · 15)`; stop if `min ≤ 0`.
2. `abs = |badlands_pillar_roof(0.75x, 0, 0.75z) · 1.5|`;
   `floor = Mth.floor(64 + min(min² · 2.5, ceil(abs · 50) + 24))`; stop if `i4 > floor`.
3. Scan `y = floor` down to `−64`: stop the scan at the first state of block `stone` (the default
   block's block); **abort the prepass** at the first state of block `water`.
4. Fill: `y = floor` down while `y ≥ −64` and the state is air: write the default block (stone).

`floor ≤ 64 + 250 + 24 = 338` (`badlands_pillar_roof` `max_value` 10/3); positions above 319 read void
air and their writes are dropped, so the runtime starts both loops at `min(floor, 319)` — result-
equivalent by source review, and pinned by a unit test with a synthetic floor > 319 (C5). No random is
drawn. Genuine branches: 244 / 113 / 2 / 53 (§2.3).

### 6.2 Frozen-ocean postpass (iff column biome is `frozen_ocean` or `deep_frozen_ocean`)

1. `min = min(|iceberg_surface(x,0,z) · 8.25|, iceberg_pillar(1.28x,0,1.28z) · 15)`; stop if `min ≤ 1.8`.
2. `abs = |iceberg_pillar_roof(1.17x,0,1.17z) · 1.5|`; `m1 = min(min² · 1.2, ceil(abs · 40) + 14)`;
   if `temperatures.sample(biome, (x, sea, z), sea).should_melt_iceberg_slightly()` then `m1 −= 2`
   (`biome` is the column biome, not re-resolved at `sea`).
3. If `m1 > 2`: `d3 = sea − m1 − 7`, `m1 += sea`; else `m1 = 0`, `d3 = 0`. `d4 = m1`.
4. `rs = root.at((x, 0, z))` (the GEN-05.9 positional root, the same factory as the surface-depth
   draw; a fresh source): `i = 2 + rs.next_int_bounded(4)`, `i1 = sea + 18 + rs.next_int_bounded(10)`,
   `i2 = 0`.
5. For `y` from `max(i4, (int) m1 + 1)` down to `min_surface`:
   `hit = (air(y) && y < (int) d4 && nd() > 0.01) || (water_block(y) && y > (int) d3 && y < sea &&
   d3 ≠ 0 && nd() > 0.15)` with Java short-circuit order (at most one draw per `y`) and
   `nd() = f64::from(rs.next_double() as f32)` (Paper build 60, P1). On `hit`: if `i2 ≤ i && y > i1`
   write `snow_block` and `i2 += 1`, else write `packed_ice`.

`(int) m1 + 1 ≤ 278` (`iceberg_pillar_roof` `max_value` 10/3) and `i4 ≤ 320`; iterations above 319
read void air and draw nothing; iterations below −64 can draw but every write is dropped and the
source is discarded. The runtime therefore iterates only `[max(min_surface, −64), min(start, 319)]` —
result-equivalent by source review, pinned by unit tests with synthetic `min_surface < −64` and
`start > 319` (C5). Genuine branches: `min ≤ 1.8` 830, iceberg 304 (melt true 287, false 17), flat 1.

### 6.3 No other write

There is no further block write outside the rule tree; `topMaterial` (carvers) is out of scope.

## 7. Chunk mutation contract

- **Replaceable positions (original):** rule tree — only positions whose state at the loop visit is
  exactly the default block state; eroded — only air positions, contiguous from `floor` downward,
  with the default block; frozen — air below `(int) d4` or `water` blocks inside `((int) d3, sea)`,
  with `snow_block` / `packed_ice`. Writes stay inside the centre chunk and `[−64, 319]`; neighbours,
  biomes, block entities, light, entities and structures are never touched (72/72 groups).
- **Write order:** columns in loop order; inside a column the eroded writes (descending `y`), then
  the rule writes (descending `y`, each immediately after its `evaluate`), then the frozen writes
  (descending `y`). Each write updates the live `WORLD_SURFACE_WG` with the original rule: raise on a
  non-air write at or above the top; on an air write exactly at the top rescan downward; otherwise no
  change. A rule result with a non-empty fluid (`water`) sets its post-processing flag.
- **Post-processing (policy D5):** the output flag set is the GEN-05.4 fluid bitmap OR the surface
  fluid writes. The original appends to per-section lists; the oracle shows 0 duplicates, so the set
  equals the original multiset; the original order (per section: NOISE order, then surface write
  order) is kept in the fixture for a later consumer, not in the runtime output.
- **Block entities:** the SURFACE write path neither creates nor removes block entities, and no
  admitted input carries one; the output has no block-entity map. Installing the result into a live
  `ferrumc_world::Chunk` (whose block-entity map and dirty sets exist) is activation and out of scope.
- **No partial result (policy D3):** every `build_chunk` call first marks the caller's buffer
  `Empty`; all refusals of §9 happen before any column work; a defensive failure inside the loop
  leaves the buffer `Empty`. Only a completely driven chunk is published (`Published` with its chunk,
  seed and settings). The original is not transactional (an exception would leave a partial proto
  chunk); all-or-nothing is FerrumC policy.

## 8. Driving the GEN-05.9 API

- The driver creates exactly **one fresh `SurfaceRuleSession` per chunk** with
  `SurfaceRuleWorkBudget::new(SURFACE_RULE_OVERWORLD_CHUNK_WORK)` (45,908,736): the driver makes at
  most one `begin_column`, at most one uncached minimum and at most 384 `evaluate` calls per column,
  each admitted by GEN-05.9 §8, so `WorkBudgetExceeded` from the session is unreachable and the
  GEN-05.9 continuation rule (restart a column in a new session) never applies. A session refusal
  inside the loop is treated as a defensive failure (`Empty`, D3).
- Per column: one `begin_column`; `evaluate` only for default-block positions with P4-admitted inputs
  (always true, §4); `min_surface_level()` once for frozen columns (0 units when already cached).
- **Replay classes** for the runtime's original-equality tests (fixture §12): `full` (whole chunk,
  exact trace equality), `refused:<kind>` (only the typed refusal, `Empty` buffer, no original value
  compared) and Vanilla cells (never expectations). There is no `partial` class: every chunk is
  recorded from its first column. The class follows mechanically from the policy table below and is
  derived on three routes (§12).

| Policy (order) | Domain | Oracle groups outside | Expected |
| --- | --- | --- | --- |
| resolver source (P6) | `MultiNoise(minecraft:overworld)`: presets normal, large_biomes, amplified | `policy/single_biome_surface/0` (genuine fixed-plains preset) | `Rules(InconsistentResolver(Source))` from session admission |
| centre domain (D4) | `cx, cz ∈ [−1874999, 1874998]` (complete 3×3 inside the chunk domain `[−1875000, 1874999]`) | `outside/normal/0/+x` (cx 1874999) | no admitted 3×3: GEN-05.5 sampling of the halo chunk 1875000 refuses `NoiseChunkErrorKind::ChunkOutOfRange` and a complete-halo GEN-05.8 resolver `StoredBiomeErrorKind::CoordinateDomain`; the driver is unreachable and nothing is published |
| P4 per call | `y ∈ [−64, 319]`, stone `1..=4096`, water ≠ `Some(i32::MIN)` | none (0 of 2,318,375 raw Paper calls of all groups) | — |

Carried GEN-05.9 runtime-review conditions: **RR-H1** — a session test that combines a wrong seed with
wrong settings pins `ResolverMismatch::Seed` first (mutant R01). **RR-H2** — a test pins the
laziness of the biome memo through `work_used`: for every admitted fixture call the resolver is
consulted exactly when a `biome` node or the `temperature` singleton is computed (mutant R06). Both
belong to the runtime ticket (§16).

## 9. Errors, refusal order and policies

`SurfaceColumnErrorKind` (optional chunk/column/block/limit/count payload, no message allocation):
`InvalidBudget`, `WorkBudgetExceeded`, `NativeLimit`, `InconsistentBinding`
(`BindingMismatch::{Seed, Settings}`), `InconsistentMaterial(MaterialMismatch::{Seed, Settings,
Geometry, Chunk})`, `Rules(SurfaceRuleErrorKind)`, `Noise`, `Temperature`, `UnclassifiedState`.
There is no driver-level centre-domain kind: a resolver for a centre outside D4 cannot be built
(§8), so such a check would be unreachable. Each kind's rustdoc states whether it is policy, limit,
consistency or defensive.

Order of `build_chunk` (every step before any column work, buffer already `Empty`):

1. `InconsistentBinding`: the column binding was bound from the given `SeededSurfaceRules` (seed,
   settings).
2. `InconsistentMaterial`: seed, settings resource, geometry `(−64, 384)`, chunk = resolver centre.
3. Session admission (GEN-05.9 order: seed, settings, source, geometry, zoom, then its 3×3 chunk
   admission) → `Rules(kind)`.
4. `WorkBudgetExceeded` if the driver allowance is below the chunk requirement (§10).
5. Loop. `UnclassifiedState` (a state outside the closed alphabet, D1), `Noise`, `Temperature` and
   any `Rules` error from the session are defensive (unreachable for admitted inputs) → `Empty`.

Construction: `SeededSurfaceColumns::bind(&SeededSurfaceRules, SurfaceColumnBindBudget)`: budget
domain, then the bind census (§10), then sampler construction and the retained-byte cap; failure
publishes nothing. The binding inherits P1 (Paper numerics), P2 (flat bedrock disabled) and P3
(three overworld settings) from the given `SeededSurfaceRules`.

FerrumC policies of this ticket (not original behaviour):

- **D1 closed state alphabet.** Classification is defined only on the 35 states the inputs and writes
  can contain: the GEN-05.4 overworld material set (air, stone, water, lava, copper_ore,
  raw_copper_block, granite, deepslate_iron_ore, raw_iron_block, tuff), the 24 rule-result states,
  the 7 band states and `packed_ice`, `snow_block`. Air class = air, cave_air, void_air; fluid =
  dense water/lava ranges; stone block = `stone`; water block = the water range. Anything else is a
  defensive `UnclassifiedState`, never inferred from solidity (as GEN-05.4).
- **D2 chunk-level pre-check** of the driver allowance (§10) instead of per-column refusals.
- **D3 all-or-nothing publication** into a caller buffer that is `Empty` on every refusal.
- **D4 centre domain** `[−1874999, 1874998]`: enforced by the producers (the 3×3 halo must lie in the
  admitted chunk domain), as by the GEN-05.9 session's 3×3 admission; the original generates there.
- **D5 outputs:** `WORLD_SURFACE_WG` and the fluid flag set are published; `OCEAN_FLOOR_WG` and the
  post-processing order are not (both derivable or kept in the fixture).
- Inherited: P1, P2, P3, P4, P6 of GEN-05.9.

## 10. Capacities, work and allocation

**Driver work units** (separate from the GEN-05.9 session allowance): 1 per loop step (one visited
`y` in any loop or scan), per noise sample, per temperature sample, per random draw, per biome
lookup and per block write; heightmap raises are part of the write.

| Part | Bound per column |
| --- | --- |
| column biome | 1 |
| eroded prepass: 3 noises + 384 scan + 384 fill + 384 writes (clamped, §6.1) | 1155 |
| column loop: 385 visits (`i5 ≤ 320` to −64) + stone-below search ≤ 385 (Σ run lengths + #runs ≤ 385) + 384 rule writes + air-top rescans ≤ 384 (disjoint ranges) | 1538 |
| frozen postpass: 3 noises + 1 temperature + 2 int draws + 384 iterations + 384 double draws + 384 writes | 1158 |
| **`W_col`** = 1 + max(eroded, frozen) + loop (they exclude each other by biome) | **2697** |

Chunk requirement `R = C0 + 256 · W_col` with `C0 = 98,304` (state copy) + `1,536` (fluid words) +
`98,304` (initial height derivation, worst case) = `198,144`, so **`R = 888,576`**.
`SurfaceColumnWorkBudget::new` admits `1..=1,048,576` (tests 0 / 1 / 1,048,576 / 1,048,577);
`build_chunk` refuses `WorkBudgetExceeded` at `R − 1` and admits `R` and `R + 1` (N−1/N/N+1). The
runtime computes `W_col` and `R` from these formulas, pins them as constants, asserts actual per-
column usage `≤ W_col` on every fixture column and reports the observed maximum.

`SurfaceColumnBindBudget` admits `1..=65,536`; bind census = six-noise Paper basis 8320 + alphabet 35
= **8355** (tests 8354 / 8355 / 8356).

Capacities (design caps; the runtime measures exact retained bytes, pins them and tests `NativeLimit`
at N−1/N/N+1 through private-cap constructors; a cap change is a Root decision):

| Item | Cap | Design figure |
| --- | --- | --- |
| `SeededSurfaceColumns` retained | 16 KiB | six Paper noises 10240 B + alphabet + header |
| `SurfaceChunkBuffer` retained | 448 KiB | 98,304 states × 4 = 393,216 B + 1,536 × 8 = 12,288 B + heights 1,024 B + header |
| per-chunk scratch | stack only | session (400 B, GEN-05.9) + one column of 384 states (1,536 B) + loop locals |

**Allocation:** `SeededSurfaceColumns::bind` and `SurfaceChunkBuffer::new` allocate (counted, not
claimed free; the buffer exactly two boxes). `build_chunk` makes **zero** heap allocations; measured
in a separate cold Debug/Release process with a positive 4096-byte control, as in GEN-05.7/.8/.9. No
TPS, RSS or speed claim follows.

## 11. API candidate and runtime ownership

One new submodule `surface_rules::column` in `ferrumc-world` (it can use the `pub(super)` session and
binding internals; no new `pub mod` in `lib.rs`), re-exported from `surface_rules`, private fields,
rustdoc on everything:

- `SurfaceColumnBindBudget`, `SurfaceColumnWorkBudget` (`new(u32) -> Result`, §10);
- `SeededSurfaceColumns::bind(&SeededSurfaceRules, SurfaceColumnBindBudget)`;
- `SurfaceChunkBuffer::new()`, `state(..)`/`world_surface_heights()`/`fluid_flag(..)` accessors that
  return data only when `Published`, and `published() -> Option<(ChunkPos, WorldSeed,
  NoiseSettingsResourceId)>`;
- `SeededSurfaceColumns::build_chunk(&self, SurfaceChunkInput<'_>, SurfaceColumnWorkBudget,
  &mut SurfaceChunkBuffer) -> Result<SurfaceChunkReport, SurfaceColumnError>` with
  `SurfaceChunkInput { rules, resolver, temperatures, corners, material }`;
- `SurfaceChunkReport`: `work_used`, maximum column work, write counts.

Crate-private additions: `WorldSurfaceHeights::set` (§5), a session accessor if needed; no change to
GEN-04/.3/.4/.5/.7/.8 behaviour, no Registry change, no dependency. Fixture path:
`server/crates/ferrumc-world/tests/fixtures/surface-column-original-1.21.8/` (the 8 files of §12,
unchanged); reference tooling (helper, runner, guards, projection, prepare, fixture generator) under
`server/crates/ferrumc-world/tests/reference/surface-column/`. `INVARIANTS.md` and `README.md` get a
column-driver section.

## 12. Oracle protocol, reproducibility and fixture

Pins and guards: `oracle/prepare.py` writes `input-pins.json` (2123 inputs: archive, artefacts, JDK,
mappings, accepted GEN-05.8/.9 references, sources, design, oracle). `oracle/run_oracle.py OUT`
refuses the five Java environment variables, helper-package shadows in the original JARs, a
noncanonical helper, byte or membership drift (before and after) and nonfresh or out-of-area output;
it checks the freshly compiled helper inventory (8 classes) and CodeSources in the JVM.
`oracle/check_guards.py OUT REF` exercises 16 guards in the caller's Python mode with explicit
`require` (5 environment guards through the real entrypoint, canonical source, digest, shadow, fresh
output, write scope, JDK/archive/design membership, 3 real-JVM CodeSource mismatches refused before
any sample). Normal and `-O` `guards.json` differ only in `entrypoint_python_optimize`. Timestamped
logs are listed only in `diagnostics-manifest.json`.

**Compact fixture** (`fixture/surface_column_fixture.py generate RAW OUT`, 8 files, 3.4 MB):
`states.tsv` (the 32 observed states, ordered by raw id, with the original flags), `biomes.tsv`, `groups.tsv`
(class, corners, seeds, Paper array digests), `chunks.tsv` (Paper S0 RLE, S0 heights and
post-processing; final `WORLD_SURFACE_WG`, `OCEAN_FLOOR_WG`, ordered post-processing and the final
states SHA-256 in material order), `columns.tsv` (per column: biome, `i4`, eroded writes, `i5`,
surface depth, call runs, outcomes, steep, minimum surface, frozen writes), `noises.tsv` (2602
columns: all eroded/frozen-biome columns plus the diagonal of every group, Paper and Vanilla bits,
Paper draws, melt), `vanilla.tsv` (Vanilla final digests, differing columns) and `manifest.json`
(policy table, replay rule, counts, labels).

`validate RAW FIX` regenerates byte for byte, then on **route 2** from the projected tokens alone:
re-derives every class from the policy table, checks every call of a `full` group against P4,
expands every column back to the raw tokens, reconstructs every final chunk from S0 plus the trace
and compares the digest, recomputes both final heightmaps and the ordered post-processing, and
expands every noise row and Vanilla digest. Normal = `-O`: 72 groups (70 `full`, 1
`refused:CentreOutsideDomain`, 1 `refused:InconsistentResolver(Source)`), 18432 columns, 2,261,637
`full` calls inside P4, 72 reconstructed digests, 2602 noise rows. **Route 3** (`delivery/freeze.py`)
derives the classes straight from the raw Paper groups and recounts P4 over all 2,318,375 raw calls.
Ten single-cell mutations of fixture copies (call outcome, call input, class, eroded write, frozen
write, steep heights, final digest, S0 run, post-processing order, noise bit) are each refused by
route 2 with the expected reason (`delivery/mutations/`, `final-compare.json`).

Runtime replay rule: for every `full` group drive the whole chunk from the FerrumC producers (the
GEN-05.4 material must equal the S0 tokens, the GEN-05.5 arrays the digests) and compare the final
digest, `WORLD_SURFACE_WG`, the fluid flag set (= final post-processing positions) and the
`cfg(test)` per-column trace exactly; for every `refused` group expect only the typed refusal and an
`Empty` buffer; never add rows, relax a class or compare a refused group's original values. Separately
compare the six noises and the extension draws bit for bit (Paper; Vanilla as control).

## 13. Policy decisions requested from Root

D1-D5 as stated in §9; P1/P2/P3/P4/P6 are inherited unchanged. Recommendations: accept D1 (closed
alphabet, consistent with GEN-05.4), D2+D3 (the session allowance proof makes mid-chunk exhaustion
impossible, so all-or-nothing costs nothing), D4, and D5 (the SURFACE step does not read
`OCEAN_FLOOR_WG`; publishing it would need a `blocksMotion` table that the Registry does not carry).

## 14. Missing data and prerequisites

**No prerequisite ticket outside `ferrumc-world` is needed** for this driver: the six noise
parameters, all 35 alphabet states and the air/fluid classification exist in the pinned Registry
(probe §2.4); every input comes from accepted World producers. For later consumers (not this
ticket): an exact `blocksMotion` table for `OCEAN_FLOOR_WG` (the coarse `is_solid_cube` agreed on all
32 observed states, which proves nothing beyond them) and a general fluid-state table for carvers
and features.

## 15. Open points and limitations

- No genuine column with a post-NOISE top at Y 319 (`i4 = 320`) was found, even in 24 amplified
  peak candidates; the void-air read at 320 and the resolver at y 320 are source-based (C5).
- No genuine steep test read a height changed earlier in the same chunk (12 candidates); time
  dependence of the view is source-based (C3).
- Eroded and frozen loop clamping to `[−64, 319]` is result-equivalent by source review; no genuine
  floor above 319 or minimum surface below −64 occurred (C5).
- Not executed: a real `WorldGenRegion`, `ServerLevel` tick, STRUCTURE_STARTS with structures
  enabled (`generateStructures = false` is a genuine world option; structures are out of scope),
  carvers. The oracle level height is `LevelHeightAccessor(−64, 384)`, equal to the overworld level.
- Vanilla history environment: single worker, cold RTree history per generation (GEN-05.8 A5).
- Non-authoritative, excluded from the freeze: `scratch/` (development runs, the superseded first
  fresh run, `superseded-run1/`), `probe/archive/`, `probe/target/`.

## 16. Conditions for the runtime ticket

- **C1 — replay** exactly as §12 for 70 `full` and 2 `refused` groups; no class relaxed.
- **C2 — producers:** the runtime test builds every input from GEN-05.3/.4/.5/.7/.8/.9 and checks
  them against the fixture tokens before driving.
- **C3 — steep timing:** a controlled chunk in which an eroded or air write changes a neighbour's or
  the current column's height before a later column's first steep test; the result must follow the
  live view.
- **C4 — direct bits:** six noises and the extension draws bit for bit against `noises.tsv`, with the
  Vanilla basis as discriminating control (outcomes cannot discriminate, §2.2).
- **C5 — source-based edges:** synthetic tests for `i4 = 320`, eroded floor > 319, frozen minimum
  surface < −64 and start > 319, an air write at the column top (rescan), water abort and every
  extension branch.
- **C6 — work and capacities:** §10 constants, budget domains, `R − 1/R/R + 1`, census N−1/N/N+1,
  retained-byte caps, zero allocations with positive control.
- **C7 — refusals:** every §9 kind in the stated order, buffer `Empty` after each.
- **RR-H1 / RR-H2** (carried from the GEN-05.9 runtime review), §8.
- **C8 — ownership/docs** as §11.

## 17. Fresh runs and freeze

```text
python3    oracle/prepare.py
python3    oracle/run_oracle.py final-normal
python3 -O oracle/run_oracle.py final-optimized
python3    oracle/check_guards.py guards-normal    final-normal
python3 -O oracle/check_guards.py guards-optimized final-normal
python3    fixture/surface_column_fixture.py generate final-normal    fixture/surface-column-original-1.21.8
python3 -O fixture/surface_column_fixture.py generate final-normal    fixture/generate-optimized
python3    fixture/surface_column_fixture.py generate final-optimized fixture/generate-from-oracle-optimized
python3    fixture/surface_column_fixture.py validate final-normal fixture/surface-column-original-1.21.8
python3 -O fixture/surface_column_fixture.py validate final-normal fixture/surface-column-original-1.21.8
python3    probe/extract.py final-normal probe/input && (cd probe && python3 run_probe.py input final)
python3    delivery/forbidden_claims_check.py > delivery/forbidden-claims.json
python3    delivery/freeze.py
```

`delivery/freeze.py` re-verifies the pins, compares both fresh runs (every output and the manifest
byte-identical), the guards, the three fixture generations, both validations, route 3, the ten
mutation controls, the probe receipt and the forbidden-claims scan (0 findings), then writes the
whole-file `delivery/freeze-manifest.json`. Freeze and brief SHA-256 are reported to Root separately.
