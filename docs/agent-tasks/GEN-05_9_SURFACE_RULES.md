# GEN-05.9 — overworld surface rules: compiled contract and genuine original oracle

Preparation delivery, round 3, 11.10.2026. The contract and oracle review belongs
to Root. This document does not authorize a runtime. Only this document and
`.codex-tmp/gen05_9/**` were written. The agent did no Git work, edited no
production file, added no dependency, started no other agent and changed no
historical GEN-05.x artefact.

Round 2 applied Root's decisions on the independent review
`GEN-05_9_SURFACE_RULES_REVIEW.md` (verdict FINDINGS, blocking B1, conditions
A1–A8, hints H1–H11). The round-1 candidate is preserved unchanged in
`.codex-tmp/gen05_9/round1-candidate/` (brief SHA-256 `59e62a77…`, freeze
`74bfa925…`, `SHA256SUMS`). §15 maps every round-2 change to its finding.

Round 3 applies Root's decisions on the second review
`GEN-05_9_SURFACE_RULES_REVIEW_ROUND2.md` (verdict FINDINGS, blocking R2-B1,
medium R2-M1, hints R2-H1–R2-H5). It makes **no new oracle run**: the round-2
raw outputs are consumed byte-unchanged. Only the replay classification
(`replay.tsv`), the fixture manifest and this brief change, plus one design-only
probe (§2.4). The round-2 candidate is preserved unchanged in
`.codex-tmp/gen05_9/round2-candidate/` (brief `496a3ddd…`, freeze `b4adf851…`,
`SHA256SUMS`). Earlier evidence and both review files were not changed; every
round-3 run wrote to new paths under `.codex-tmp/gen05_9/round3/`. §16 maps
every round-3 change to its finding.

## 1. Basis, scope and consumed producers

Root's readonly archive of main `9a2adfd293c975cefa070ac3073a4eefc46ceaf4`
(`.codex-tmp/gen05_9-root-base/archive`) matches all 1292 entries of
`base.freeze.json`, SHA-256
`c9a6fd4ab13de587499219dee93e0f8c48f5c26eada956121b7a6fb8ac314c0e`.
`oracle/prepare.py` re-hashed every entry before pinning. Code was read only
from that archive. The shared tree's DUR-03.3 storage WIP was ignored.

Original artefacts are the ones the accepted GEN-05.8 oracle already pins
(`.codex-tmp/gen05_8/oracle/input-pins.json`, SHA-256 `bffbed18…50d8`, freeze
`d838de6c…6cee`). Every byte was re-verified, not copied by trust:

- Vanilla 1.21.8: inner JAR `2bee159e…0250` and 44 libraries.
- Paper build 60, commit `29c8822d90899c89d2689338e81a98f690bcba12`: inner JAR
  `c64fb7db…1507` and 119 libraries.
- The complete JDK 21.0.12.1 (454 files).
- The official mappings `server-mappings.txt`.

No new original artefact was downloaded.

This ticket covers three parts: the compiled original overworld rule tree,
its seed binding, and the evaluation of one block position against an
explicit per-chunk, per-column and per-block context. The `SurfaceSystem`
column driver comes later and is not part of it. That driver covers stone
depth, water height, the eroded-badlands prepass, the frozen-ocean
postpass, block writes and fluid post-processing. Chunk activation, carvers,
features, Nether/End rules, Survival and Java plugins are also out of scope.

The future runtime consumes these accepted producers unchanged:

| Producer | Use |
| --- | --- |
| `ferrumc-registry::worldgen::NoiseSettingsResourceId::definition()` | exact original overworld settings JSON (`surface_rule`, `sea_level`, `noise.min_y/height`, `legacy_random_source`) |
| `ferrumc-registry::worldgen::NoiseResourceId::parameters()` | octave/amplitude parameters of the nine bound noises |
| `ferrumc-registry::block_state::compute_state_id` | result-state resolution (Name + Properties, unspecified properties from the default state) |
| `ferrumc-registry::biome::BiomeResourceId::from_name` | biome condition sets |
| GEN-04 `random::{WorldgenRandomSource, PositionalRandomFactory}` | positional root, named forks, `at(x,y,z)`, `next_float`, `next_double` |
| GEN-04 `noise::NormalNoise` and the crate-private `paper_build60_basis` | the nine NormalNoise samplers with the Paper offset basis |
| GEN-05.7 `biome_temperature::BuiltinBiomeTemperatures::sample` | `temperature` condition (`cold_enough_to_snow`) |
| GEN-05.8 `stored_biome::SurfaceBiomeResolver::biome_at_block` | every biome lookup; no other biome path exists |
| GEN-05.3 `NoiseChunkSession::preliminary_surface_level` | the four preliminary corners. These come from the later column driver, not from this evaluator |

## 2. Original facts

### 2.1 Source review (read and reconstructed, not executed)

`sources/collect.py` copied whole Base (paperweight mache) and Applied files,
every per-file source patch, three feature patches, the Paper-only classes and
both original JARs' `noise_settings` resources. The copies are byte-pinned in
`sources/source-map.json` (123 entries). The closure covered is:

- `SurfaceRules`, `SurfaceRuleData`, `SurfaceSystem`, `VerticalAnchor` and
  `WorldGenerationContext`;
- `NoiseGeneratorSettings`, `RandomState`, `Noises` and `NoiseChunk`;
- the Xoroshiro/Legacy random sources, `RandomSupport`,
  `NormalNoise`/`PerlinNoise`/`ImprovedNoise`, `Mth` and `CaveSurface`;
- `Biome`, `BiomeManager`, `BlockPos`, `Heightmap`, `ChunkAccess`/`ProtoChunk`
  and `BlockState`;
- `Level`, `Bootstrap` and `CarvingContext`.

Findings:

1. `SurfaceRules`, `SurfaceRuleData`, `SurfaceSystem`, `VerticalAnchor`,
   `NoiseGeneratorSettings`, `RandomState`, `Noises`, `NoiseChunk`, all random
   sources, the noises and `Mth` are byte-identical in Base and Applied. No
   feature patch touches them.
2. **Paper changes the data, not the classes.** The resource patches
   `patches/resources/.../noise_settings/{overworld,large_biomes,amplified}.json.patch`
   replace the bedrock-floor `minecraft:vertical_gradient` with
   `paper:optionally_flat_bedrock_condition_source` (`is_roof:false`).
   `caves` and `nether` additionally get a roof variant. That condition source
   (`io.papermc.paper.world.worldgen.OptionallyFlatBedrockConditionSource`,
   registered by the Paper `Bootstrap` patch) reads
   `context.context.level().paperConfig().environment.generateFlatBedrock`.
   The default is `false`. When enabled, both bounds collapse to
   `min(true,false)` (floor) or `max(true,false)-1` (roof). The Paper
   `WorldGenerationContext` patch adds the `Level` carrier and throws a
   `NullPointerException` when it is absent. This is source review only; the
   enabled branch has no valid differential evidence (§2.2, B1).
3. **Static bytecode closure differential** (`bytecode/compare.py` →
   `bytecode/closure-compare.json`). The 103 closure classes were compared
   (1353 members) through javap. Constant-pool indices were removed, Vanilla
   names translated through the official mappings and branch targets
   rewritten as instruction indices. A control-flow-insensitive multiset of
   arithmetic opcodes and literals was also compared. That multiset differs in
   only three places:
   - `XoroshiroRandomSource.nextDouble`: Vanilla `l2d/dmul 2^-53`, Paper
     `l2f/fmul 1.110223E-16f/f2d`;
   - `BiomeManager.getFiddle`: the known GEN-05.6 divergence;
   - literal constants inlined by Paper in `BlockPos`, plus codec range
     constants in `VerticalAnchor$*`, with equal values.

   `nextFloat`, the noise sampling arithmetic, `Mth.map/lerp2/floor`, every
   `SurfaceRules` condition, `SurfaceSystem.getSurfaceDepth` and `getBand`
   carry equal arithmetic. `Biome.getTemperature` differs in control flow only:
   Moonrise removed the Vanilla per-thread cache (GEN-05.7). This is
   static-analysis evidence. The execution below confirms it.
4. Laziness, read from source:
   - `LazyXZCondition` invalidates on `updateXZ`: `noise_threshold`, `steep`,
     `hole`. `getSurfaceSecondary` and `getMinSurfaceLevel` are also cached
     per column, and the preliminary corners are cached per 16×16 surface
     cell.
   - `LazyYCondition` invalidates on `updateY` *and* `updateXZ`: `biome`,
     `y_above`, `water`, `stone_depth`, `vertical_gradient`, the Paper flat
     variant and `temperature`.
   - The biome is a per-block `Suppliers.memoize`.
   - `AbovePreliminarySurfaceCondition` and `NotCondition` are not cached.
   - `temperature`, `steep`, `hole` and `abovePreliminarySurface` are single
     condition objects per context, shared by every occurrence. All other
     conditions are separate objects per tree occurrence, created by
     `RuleSource.apply`.
5. `SurfaceSystem` creates the per-block column state itself (stone depths
   and water height in `buildSurface`, lines 106-156 of the pinned copy). It
   also calls `updateXZ` once per column and only calls `tryApply` for blocks
   equal to the default block. `topMaterial` (carvers) uses the same entry
   points with `updateY(1, 1, hasFluid ? y+1 : MIN, …)`.

### 2.2 Differential evidence (actual original execution)

The freshly compiled, JDK-only helper `oracle/SurfaceRulesOracle.java` drives
actual original objects by reflection. Vanilla is reached through the pinned
official mappings, Paper through Mojang names. CodeSources of original
classes, libraries and helper are checked inside the JVM. One run has three
JVM stages:

1. **scan (Paper)**: deterministic lattice search over genuine
   `MultiNoiseBiomeSource.getNoiseBiome` outputs (seed 0, normal preset).
   - ring/x/z order, 256-block step, radius 49152, y ∈ {64, 120, 176, 0, -40};
   - 35 declared target biomes;
   - two extra hits for 14 coverage biomes;
   - a hole search: the first column with genuine
     `SurfaceSystem.getSurfaceDepth ≤ 0` in five biomes;
   - six declared noise-band searches (five found) with the genuine
     `RandomState.getOrCreateNoise(...).getValue`.

   Group selection therefore uses original outputs. Every expectation is still
   an original outcome.
2. **vanilla** and 3. **paper**: for each of 80 groups, actual
   `WorldPreset`/`RandomState`/`ProtoChunk`, nine genuine `createBiomes`
   chunks and genuine `fillFromNoise` on the centre. The RTree history starts
   cold per group, and the nine chunks share it in row-major order. Each group
   then runs three tiers:
   - **G (genuine context):** one genuine
     `SurfaceSystem.buildSurface(randomState, biomeManager, …, ruleSource)`
     per group (80 per mode). Its `RuleSource` argument is a recording proxy.
     The proxy applies the genuine tree to the genuine `Context` that
     `buildSurface` created and records every `tryApply` that `buildSurface`
     makes. Stone depths, water height, surface depth, heights and biome
     manager are all produced by original code. 2877604 Paper `tryApply`
     calls were made: every one enters a SHA-256 digest, and 133735 are
     written (declared windows; each recorded window is a prefix of its
     column).
   - **C (controlled per-block inputs):** a genuine `SurfaceRules.Context`
     with genuine system, noise chunk, chunk and the
     `biomeManager::getBiome`-equivalent delegate. Declared per-block values
     go through the original protected `updateXZ`/`updateY`, the entry points
     `buildSurface` and `topMaterial` use. Per-column state stays genuine.
     These rows are labelled controlled. No stone depth or water height is
     presented as `SurfaceSystem`-produced. The steep probe sets stone through
     genuine `ProtoChunk.setBlockState`, whose original heightmap update
     produces the heights.
   - **D (diagnostics).**

Every row records:

- the original outcome, or the exception;
- the context fields;
- each lazily computed condition with its original result;
- the exact noise value, read from the noise object the condition captured;
- the gradient `nextFloat`, read from the captured positional factory;
- the shared condition objects and their lazily computed values (steep
  heights at evaluation time, minimum surface with corners, secondary noise);
- the memoized biome;
- the per-column genuine surface-noise value and positional `nextDouble`;
- a direct genuine `BiomeManager.getBiome` query.

Only side-effect-free reads are added. The `above` probe is taken only when the
minimum surface level is already cached.

Every C row is also evaluated on two separate contexts: one for the bootstrap
tree (`SurfaceRuleData` objects) and one for the FerrumC registry tree (its
exact export slice decoded by the original codec). Both receive `updateXZ` on
every column change, because the shared column key is set only after all
probes of a row (review §2).

**Flat-bedrock probe: invalid, not authoritative (review B1).** In Paper, every
C row with `y ≤ 10` was additionally evaluated under
`generateFlatBedrock=true`, reached through a `sun.misc.Unsafe`-allocated
`ServerLevel` configuration carrier. The helper decides whether a probe needs
`updateXZ` from the shared `Controlled.column` key
(`oracle/SurfaceRulesOracle.java:605-610`), which is set after every row
(`:630`), while the flat context is touched only for `y ≤ 10` (`:622-629`). The
first row of every controlled instance has `y > 10`, so the flat context never
received `updateXZ`: it evaluated with block X/Z 0, surface depth 0 (therefore
`hole = true`) and the initial `lastUpdateXZ`. Neither `buildSurface` nor
`topMaterial` produces that state; it is a fabricated column state. All 13,608
recorded flat cells (`paper/rows.tsv` column `paper_flat_bedrock`), the counts
`paper_flat_compared`/`paper_flat_differ` and the report entry
`paper_flat_bedrock_true_vs_default` are therefore invalid and not
authoritative. They stay byte-unchanged in the frozen outputs, because
deleting them would break the reproducible manifests. The round-2 fixture
manifest and freeze manifest mark them explicitly as invalid. No contract
statement, expectation or policy relies on them. The `Enabled` mode has no
valid differential evidence (open point, §12).

**Results** (fresh round-2 runs, byte-identical to round 1; normal = `-O`):

| Item | Vanilla | Paper |
| --- | --- | --- |
| Rows (G / C / D) | 133736 / 632096 / 1 | 133735 / 631401 / 2 |
| `buildSurface` calls / `tryApply` calls from them / recorded | 80 / 2877606 / 133736 | 80 / 2877604 / 133735 |
| Original exceptions in G/C rows | 0 | 0 |
| Cross-tree comparisons, mismatches | 1264192, 0 | 1262802, 0 |
| Lazy condition nodes with both outcomes | 127 / 127 | 127 / 127 |
| Shared-condition outcomes `above/hole/steep/temperature` (true/false) | above 650968/114864, hole 280/2491, steep 235/3294, temperature 32/970 | above 650470/114666, hole 280/2491, steep 235/3294, temperature 32/970 |
| Flat bedrock `true` probe (C rows, `y ≤ 10`) | — | 13608 cells recorded, **invalid, not authoritative (B1)**; no claim |

Trees (`*/tree-*.tsv`, `projection/tree.tsv`):

- The bootstrap tree, the JAR tree and the registry-slice tree are
  canonically equal in Vanilla (428 nodes, also for `large_biomes` and
  `amplified`).
- In Paper they differ at exactly node 2 (`paper_optionally_flat_bedrock`,
  bedrock floor, `is_roof=false`).
- With the Paper default `generateFlatBedrock=false` (the only mode this
  contract ships), the genuine Paper condition produced the same outcome as
  the Vanilla gradient of the bootstrap and registry trees in every
  cross-tree comparison (1,262,802 in Paper, 0 mismatches).
- Applying the Paper JAR tree without a `Level` throws the original NPE (D row).

**Vanilla vs Paper differ, and the oracle quantifies it**
(`projection/modes.json`):

- every common column's genuine surface-noise value and positional
  `nextDouble` differ: 18,984 columns, distinct per group, with a column cell
  in both originals (18,728 physical `(preset, seed, x, z)` columns, because
  one chunk lies in two groups; R2-H4, recounted in round 3); 26,556 is the
  number of common *rows* that carry such a cell;
- 510 outcomes differ (449 through the biome, all in the desert group; 59
  through the terrain height; 2 through the surface depth);
- the surface depth differs in 36 rows, all from **one** column
  (`sweep/jagged_peaks/1`, x = −1272, z = −8177: 2 → 3). The driver is the
  noise basis, not the f32 rounding of the draw: the Vanilla noise with the
  Paper draw still yields 2;
- 682 direct biome queries differ, all in `biome/desert`;
- three of 720 stored biome chunks (98 values) differ even with cold
  histories, because the climate noises use the Paper offset basis;
- 14 groups' terrain heights differ;
- gradient `nextFloat` bits, clay bands and anchors are identical. The zoom
  semantics contribute no difference: all 682 biome differences lie in the
  group whose stored array differs (GEN-05.8 H-2).

Tests that presuppose a Vanilla/Paper difference are therefore justified for
noise values, surface depth, stored biomes and the bedrock data node. They are
*not* justified for gradient draws, clay bands, anchors or the rule
structure.

### 2.3 Design probe (private archive copy + one example file; design only)

`registry-probe/run_probe.py` built `surface_design_probe.rs` in Debug and
Release. The outputs are identical
(`registry-probe/final-probe/{debug,release}.tsv`). Results:

- **No Registry prerequisite.** All 100 block rules resolve through
  `compute_state_id` to the original `Block.getId` (24 distinct states). All
  27 biome names and all 7 threshold noises resolve.
- The existing public sampler derivation reproduces the genuine **Vanilla**
  values bit-exactly in 26593/26593 columns:
  `WorldgenRandomSource::new(Xoroshiro128PlusPlus, seed).fork_positional()` →
  `from_hash_of(name)` → `NormalNoise::new`. This holds for the surface noise,
  `at(x,0,z).next_double()` and the surface depth. This probe uses the
  Vanilla basis only; the Paper basis is covered by §2.4.
- Gradient draws `from_hash_of(name).fork_positional().at(x,y,z).next_float()`
  are bit-exact in both originals: 1052432/1052432 / 1051413/1051413.
- **Paper `nextDouble` == `f64::from(vanilla_next_double as f32)`** in every
  genuine Paper column (26572/26572). It equals the unrounded
  value in 0 columns.
- Sizes: `NormalNoise` 128, `ImprovedNoise` 280, `PositionalRandomFactory` 24,
  `SurfaceBiomeResolver` 136, `BuiltinBiomeTemperatures` 2232 and
  `BlockStateId` 4 bytes. The nine bound noises contain 56 `ImprovedNoise`
  (15680 bytes).
- The overworld settings definition is 129,737 bytes. That is 1,335 bytes
  below the Registry per-definition cap (`MAX_DEFINITION_BYTES` 131072). Note
  it as a drift risk; it is not a defect.

### 2.4 Paper-basis probes (round 2, review A5; round 3, R2-H1; private archive copies, design only)

`round2/paper-basis-probe/run_probe.py` copies Root's archive, verifies it
against `base.freeze.json`, and adds exactly one `#[cfg(test)]` module
(`surface_paper_basis_probe.rs`). It also appends one blank line and the two
registration lines `#[cfg(test)]` / `mod surface_paper_basis_probe;` to
`ferrumc-world/src/lib.rs`. The runner refuses any other difference. It runs
the test in Debug and Release with its own target directory; both outputs are
byte-identical. The module builds the nine samplers through the public
derivation of §2.3 and converts them with the existing crate-private
`NormalNoise::paper_build60_basis`. It then compares bits against the genuine
values of both originals' rows (`round2/paper-basis-probe/final/`):

| Compared against genuine cells | Paper rows, Paper basis | Paper rows, Vanilla basis | Vanilla rows, Vanilla basis |
| --- | --- | --- | --- |
| 33 `noise_threshold` nodes, node-wise (7 noises) | 35958 / 35958 | 0 / 35958 | 35958 / 35958 |
| `surface` column noise (`SurfaceSystem.surfaceNoise`) | 26572 / 26572 | 0 / 26572 | 26593 / 26593 |
| `surface_secondary` (`getSurfaceSecondary`) | 1182 / 1182 | 0 / 1182 | 1202 / 1202 |
| column draw (Paper: f32-rounded `next_double`) | 26572 / 26572 | 0 / 26572 | 26593 / 26593 |
| surface depth | 26572 / 26572 | 26571 / 26572 | 26593 / 26593 |

Per noise in the Paper basis on Paper rows: `surface` 23104, `powder_snow`
5927, `calcite` 2425, `gravel` 1904, `packed_ice` 1430, `ice` 1147 and
`surface_swamp` 21, each fully equal. Each of the 33 nodes is listed separately
in `receipt.json`. The Vanilla basis on Paper rows (and the Paper basis on
Vanilla rows) matches no noise cell, so the comparison discriminates. The single
surface-depth mismatch under the Vanilla basis is the H1 column.

**`clay_bands_offset`: no basis evidence (corrected in round 3, R2-H1).** The
oracle never samples this noise, so no genuine value exists to compare against.
Round 2 called the `bandlands` outcomes indirect evidence. They are not evidence
for the basis: the independent review found that all 60,846 Paper rows reaching
a `bandlands` rule match the band formula in **both** bases, and that
`Math.round(4·clay)` is equal in both bases in all 3,393 badlands columns. The
round-3 design probe (`round3/clay-probe/`, same private-copy method as above,
module `surface_clay_basis_probe.rs`, Debug = Release) measures why. In the
3,393 genuine Paper badlands columns:

| Item | Result |
| --- | --- |
| f64 bits of `clay(x,0,z)`, Paper basis vs Vanilla basis | differ in 3,393 / 3,393 columns |
| band offset `Math.round(4·clay)`, Paper vs Vanilla basis | differs in 0 / 3,393 columns |
| largest value difference between the bases | 7.56e-6 (so `4·Δ` ≤ 3.03e-5) |
| smallest distance of `4·clay` (Paper basis) to a rounding boundary `k + ½` | 1.06e-4 |

The bandlands outcomes check the band table and the index formula, but they
cannot discriminate the basis. A direct bit comparison would discriminate in
every column. It needs genuine Paper `clay_bands_offset` values, which the
frozen oracle does not contain; this design round makes no new oracle run (Root
decision). The direct bit comparison is therefore a mandatory runtime condition
(§13, A5). The Paper basis of this noise currently rests only on the same
`paper_build60_basis` path that is bit-exact for the other eight noises.

## 3. Compiled form (contract)

**Data source.** `NoiseSettingsResourceId::from_name("minecraft:overworld")`
`.definition().json()`. This is the CAT-02.1 Mojang datagen export, which the
Registry build pins by SHA-256. The oracle proved it canonically equal to
both the Vanilla JAR resource and the bootstrap `SurfaceRuleData` objects.
`large_biomes` and `amplified` carry the identical tree. There is no hand
transcription.

**Compile.**

1. One bounded parse of the whole definition with World's existing checked
   JSON parser, under the Registry per-definition limits (128 KiB, 4096
   nodes, depth 32, 256 entries, 256 string bytes). Number tokens are decoded
   as original DOUBLE/INT.
2. One preorder walk builds an immutable node table. Each node holds a kind,
   its parent index, a child range or condition/then indices, and typed
   payloads:
   - `BlockStateId`;
   - a `BiomeResourceId` set;
   - a `NoiseResourceId` slot plus raw min/max f64 bits;
   - a random-name slot plus two anchors;
   - anchor plus multiplier plus the `add_stone_depth` flag;
   - offset, `add_surface_depth`, secondary range and `CaveSurface`.

   Kinds:
   - rules: `sequence`, `condition`, `block`, `bandlands`;
   - conditions: `biome`, `noise_threshold`, `vertical_gradient`,
     `paper_optionally_flat_bedrock`, `y_above`, `water`, `stone_depth`,
     `steep`, `hole`, `above_preliminary_surface`, `temperature`, `not`.

   Block, biome and noise IDs are separate types. Quart IDs never appear. A
   single-child `sequence` keeps its node but evaluates as its child.
3. **Paper data substitution (policy, mechanically checked).** Every
   `vertical_gradient` with random name `minecraft:bedrock_floor` (or
   `minecraft:bedrock_roof`) compiles to `paper_optionally_flat_bedrock` with
   `is_roof` false (or true). Its anchors and name are unchanged. This
   reproduces Paper's pinned resource patches. Because only
   `PaperFlatBedrock::Disabled` is admitted (§11, P2), the node evaluates
   exactly as the gradient. The runtime test projects the compiled table to the
   oracle's canonical columns. After substitution it must equal the fixture's
   `tree-paper.tsv` (Paper JAR form); before substitution it must equal the
   fixture's `tree.tsv` (Vanilla JAR = bootstrap = registry form). The
   difference must be exactly `paper-tree-substitution.json`. All three files
   are mechanical copies of the oracle output and are part of the fixture
   (review A2). Any other difference fails.
4. **Admission.** Only `minecraft:overworld`, `minecraft:large_biomes` and
   `minecraft:amplified` are admitted; their trees are canonically equal.
   `caves` and `floating_islands` (different trees), `nether` and `end` are
   refused as `UnsupportedSettings` (policy). `legacy_random_source` must be
   `false`.

Census of the overworld tree (428 nodes):

| Kind | Count |
| --- | --- |
| `condition` (rule) | 137 |
| `block` | 100 |
| `sequence` | 48 |
| `bandlands` | 2 |
| `biome` | 39 |
| `noise_threshold` | 33 |
| `stone_depth` | 23 |
| `water` | 20 |
| `y_above` | 10 |
| `steep` | 5 |
| `not` | 4 |
| `hole` | 3 |
| `vertical_gradient` | 2 |
| `above_preliminary_surface` | 1 |
| `temperature` | 1 |

Other figures: maximum depth 13 (root 0), largest sequence 12, largest biome
set 3 (53 biome references), 24 distinct result states, 7 threshold noises.
Sharing exists only in the bootstrap objects (180 shared positions in
`tree-bootstrap.tsv`, recounted in both modes). It is result-neutral, because
`apply` creates conditions per occurrence; the compiled form does not keep it.

**Drift detection.**

- The Registry build already pins the export (CAT-02.1 SHA-256 `bc0dbfa4…1b30`).
- The runtime asserts the exact census above.
- It compares the canonical projections with the fixture's `tree.tsv` and
  `tree-paper.tsv` (SHA-256 in the fixture manifest).
- `oracle/project_originals.py` re-proves bootstrap = registry = Vanilla JAR
  and the single Paper node on every fresh run.

## 4. Seed binding

`SeededSurfaceRules::bind(Arc<SurfaceRuleTemplate>, WorldSeed,
SurfaceNumericProfile, PaperFlatBedrock, SurfaceRuleBindBudget)` first checks
the two policy selectors:

- `SurfaceNumericProfile::PaperBuild60` is the only admitted profile.
  `SurfaceNumericProfile::Vanilla` is refused as `UnsupportedNumericProfile`
  (policy P1). A Vanilla request is never computed silently as Paper.
- `PaperFlatBedrock::Disabled` (Paper default) is the only admitted mode.
  `PaperFlatBedrock::Enabled` is refused as `UnsupportedFlatBedrock`
  (policy P2) until a separate world-configuration ticket exists. The enum
  keeps both variants so that ticket does not change the signature.

It then builds, once per seed:

- the positional root
  `WorldgenRandomSource::new(Xoroshiro128PlusPlus, seed).fork_positional()`,
  the same object as `RandomState.random`, `SurfaceSystem.noiseRandom` and
  the density root in `density/bind.rs`;
- nine `NormalNoise`, each `NormalNoise::new(&mut root.from_hash_of(key), …)`
  converted with `paper_build60_basis`: `surface`, `surface_secondary`,
  `surface_swamp`, `packed_ice`, `ice`, `powder_snow`, `calcite`, `gravel`,
  `clay_bands_offset`. One sampler per distinct key; a per-node copy is
  result-neutral. §2.4 shows this conversion bit-exact against the genuine
  Paper values for eight of the nine noises. `clay_bands_offset` has no basis
  evidence; its direct bit comparison is a runtime condition (§2.4, A5);
- two gradient factories `root.from_hash_of(name).fork_positional()`
  (`bedrock_floor`, `deepslate`);
- the 192 clay bands: `SurfaceSystem.generateBands` over
  `root.from_hash_of("minecraft:clay_bands")` with the original draw order;
- the resolved anchors: generation context
  `minY = max(level -64, settings -64) = -64`,
  `genDepth = min(384, 384) = 384`, sea level 63.

The genuine clay bands, resolved anchors and sea level of every group are in
`groups.tsv`. They are equal in both originals.

## 5. Explicit context (produced by others, consumed here)

| Scope | Field | Domain (admission) | Producer | Original |
| --- | --- | --- | --- | --- |
| session | centre `ChunkPos` | the resolver's complete-halo domain `[-1_874_999, 1_874_998]` | GEN-05.8 resolver | `chunk.getPos()` |
| session | biome resolver | complete 3×3; identity: same seed and settings, source `BiomeSourceIdentity::MultiNoise(minecraft:overworld)` (policy P6), geometry `BiomeQuartGeometry` `min_y = -64`, `height = 384` = the settings' `noise.min_y`/`noise.height` (consistency); `BiomeZoomSemantics::Paper…` (policy P6) | GEN-05.8 | `WorldGenRegion` manager over chunks of level height `(-64, 384)` |
| session | temperatures | `BuiltinBiomeTemperatures` | GEN-05.7 | `Biome.coldEnoughToSnow` |
| session | `PreliminarySurfaceCorners` | four `i32` incl. `i32::MAX` ("no surface") at block `(16cx,16cz)`, `(16cx+16,16cz)`, `(16cx,16cz+16)`, `(16cx+16,16cz+16)` | column driver via GEN-05.3 | `NoiseChunk.preliminarySurfaceLevel`, cached per surface cell |
| column | `SurfaceColumnPos` | `ChunkPos` plus local x,z `0..16`; the driver uses x outer, z inner | column driver | `updateXZ` |
| column | surface depth | computed at `begin_column` | evaluator | `getSurfaceDepth` |
| column | heights view | `&WorldSurfaceHeights` (256 × `i32`, the original `getHeight` value, i.e. top block Y or `minY-1`), passed **on every call** | column driver | chunk `WORLD_SURFACE_WG` |
| block | `y` | `[minGenY, minGenY+genDepth-1]` = `[-64, 319]` (policy) | column driver | `updateY.blockY` |
| block | stone depth above / below | `1..=4096` (policy; `buildSurface` yields `1..=height`) | column driver | `i6`, `i9-i8+1` |
| block | water height | `None` (original `Integer.MIN_VALUE`) or `Some(v)` with `v ≠ i32::MIN` | column driver | `i7` |

The driver calls `evaluate` only for default-block positions, as the
original does. Skipped `updateY` calls are result-neutral, because
Y-scoped caches are per block anyway.

The original reads steep heights **lazily at the column's first `steep` test**.
At the chunk edges the clamp reads the current column itself. A rule may
write `air` into the current column (frozen-ocean hole) before that first
test. Freezing four heights at `begin_column` would therefore be wrong in
principle. The borrowed per-call view reproduces the original exactly. G rows
record the heights read at the evaluating call (`steep_heights`). This
requirement is **source-based**: no differential case exercises it. In all 838
genuine steep tests the heights at test time equal the final heights (H6),
and controlled rows never write. The runtime therefore adds a controlled test
in which the heights view changes after a column's first steep test and the
result stays the first test's (§13, A8). A refused `evaluate` call (§7) tests
nothing and fills no cache, so the column's first steep test is the first
*admitted* call that reaches a `steep` node. The original also evaluates inputs
outside the P4 domains and can fill per-column caches there; that is why rows
after a refused row are never `full` (§6).

## 6. Exact per-condition semantics and evaluation order

Evaluation runs on the compiled preorder with an explicit fixed stack of
depth ≤ 32 and no recursion:

- `condition` tests its condition; if true, it evaluates its rule;
- `sequence` evaluates its children in order and returns the first state;
- `block` returns its state;
- `bandlands` returns `bands[(y + i + 192) % 192]` with
  `i = (int) Math.round(clayOffset(x,0,z)·4)` (`Math.round` → long → int)
  and Java `%`, which keeps the sign of the left operand. Within the `y`
  domain `[-64, 319]` the left operand is nonnegative whenever `i ≥ -128`.
  `clay_bands_offset` is a one-octave, amplitude-1.0 `NormalNoise`. Its
  original max-value bound (source-derived: both Perlin stacks `edgeValue(2.0)
  = 2`, times `valueFactor = (1/6)/0.2`) is 10/3. So `|i| ≤ 14`, far inside
  that range (H9). The runtime computes `%` with Java semantics. A negative
  index, which is unreachable for admitted inputs, is a defensive `Noise`
  error that poisons the session; it is never wrapped with `rem_euclid`. A
  unit test pins the invariant at both `y` ends.

Result: `State(BlockStateId)` or `NoRule`. All integer arithmetic is Java
`int` with wrap.

| Condition | Exact inputs | Cache |
| --- | --- | --- |
| `biome` | `resolver.biome_at_block(x,y,z) ∈ set`; one resolver call per block, shared by all biome nodes and `temperature` | per block |
| `noise_threshold` | `v = noise.sample(blockX, 0.0, blockZ)`; true iff `v ≥ min && v ≤ max` | per node, per column |
| `vertical_gradient` | `y ≤ T` → true; `y ≥ F` → false; otherwise `factory.at(x,y,z).next_float() < map(y,T,F,1,0)` with `map = lerp(inverseLerp)` in f64 | per node, per block |
| `paper_optionally_flat_bedrock` | as `vertical_gradient` (only `Disabled` is admitted; `Enabled` is refused at bind, P2) | per node, per block |
| `y_above` | `y + (add ? stoneAbove : 0) ≥ anchorY + surfaceDepth·mult` | per node, per block |
| `water` | `water = None` → true, else `y + (add ? stoneAbove : 0) ≥ water + offset + surfaceDepth·mult` | per node, per block |
| `stone_depth` | `(ceiling ? stoneBelow : stoneAbove) ≤ 1 + offset + (add ? surfaceDepth : 0) + (range == 0 ? 0 : (int) map(secondary,-1,1,0,range))`; Java `(int)` truncates toward zero and saturates | per node, per block; secondary per column |
| `steep` | local `lx = x&15`, `lz = z&15`; `h(lx, min(lz+1,15)) ≥ h(lx, max(lz-1,0)) + 4`, else `h(max(lx-1,0), lz) ≥ h(min(lx+1,15), lz) + 4` (original asymmetric axes) | shared, per column |
| `hole` | `surfaceDepth ≤ 0` | shared, per column |
| `above_preliminary_surface` | `y ≥ minSurface`; `minSurface = floor(lerp2((x&15)/16f, (z&15)/16f, c0,c1,c2,c3)) + surfaceDepth − 8` (f32 deltas widened, f64 lerps, wrapping `+`) | none (cached minimum per column) |
| `temperature` | `temperatures.sample(biome, (x,y,z), 63).cold_enough_to_snow()` | shared, per block |
| `not` | negation of the target's cached result | none |

`surfaceDepth = (int)(surface(x,0,z)·2.75 + 3.0 + paperNextDouble(root.at(x,0,z))·0.25)`,
with `paperNextDouble = f64::from(next_double() as f32)`. Only the time of the
first `steep` test changes results: the heights view at that moment counts.
Every other cache is result-neutral, because its inputs are pure. For
work-bound and trace equality the runtime nevertheless caches exactly per
node as above. The oracle's `computed` column lists every lazily computed
node and its result for each call. Under first-match evaluation that set
determines the order.

**Fixture replay rule (review A1; third class `refused` in round 3, R2-B1).**
The runtime emits the same projection in `cfg(test)` and compares it under this
mechanical rule. The fixture file `replay.tsv` states it per row (columns
`ordinal`, `run_start`, `replay`, `heights`, `refusal`). `validate` re-derives
all three classes and the refusal kind along a second route from the projected
row tokens. Every G/C row has exactly one class:

- A **column run** is the sequence of raw rows from one original `updateXZ`
  to the next on the recording context. It starts at the raw row whose
  column cell (`column_surface_noise_and_draw`, fixture `column ≠ -`) is set
  and continues over consecutive raw ordinals with equal tier, group,
  `update_xz`, `ctx_x` and `ctx_z`. `run_start` is that row's ordinal.
- **`refused`** (policy expectation, no original equality): the row's recorded
  block input lies outside a P4 domain (§5): `y` outside `[-64, 319]`, or a
  stone depth outside `1..=4096`. The column `refusal` names the expected kind,
  which is the first failing check in the `evaluate` order of §7:
  `BlockOutsideGenerationHeight` before `InvalidStoneDepth`. Water cannot be
  refused from the fixture: its `None` is the original `i32::MIN`, and every
  other recorded value is an admitted `Some`. The runtime evaluates the row
  alone after `begin_column`. It must get exactly that kind, with no outcome and
  no partial result, and without any state change. Its `cfg(test)` state
  projection (active column, surface depth, every per-column cache including
  the minimum surface and the steep heights, `work_used`) must be equal before
  and after the call. No original value of a refused row is an expectation.
  Result, trace, minimum surface and the other cells stay evidence of the
  original outside the policy domains (§7).
- **`full`** (original equality): every raw ordinal from `run_start` to the
  row is in the fixture **and admitted**; no row of the prefix is `refused`.
  The runtime calls `begin_column` once for the run and evaluates the run's
  rows in ordinal order with their recorded inputs. Result, surface depth,
  biome, the complete `computed` trace with noise and draw bits, singletons,
  steep heights, minimum surface, secondary noise and column cell must all
  equal. The `above` singleton is emitted exactly when the column's minimum
  surface is already cached at the end of the call, as the oracle records it.
- **`partial`** (evidence only for per-column values): an admitted row whose
  run prefix is incomplete or contains a `refused` row. In the second case the
  original may have computed per-column values in the refused call, and later
  rows then read them from cache. The runtime computes them only at the first
  admitted call.
  Example: the run starting at ordinal 8721 begins with four refused
  stone-depth rows, and the original computed the column minimum in 8721. The
  runtime evaluates the row alone after `begin_column`. Only
  result, surface depth, biome, the `temperature` singleton and the `computed`
  entries whose node kind (from `tree.tsv`) is per block (`biome`,
  `vertical_gradient`, `paper_optionally_flat_bedrock`, `y_above`, `water`,
  `stone_depth`, with draw bits) must equal. These do not depend on the cache
  state: per-column values are pure, and the heights view is fixed (next
  item). `noise_threshold` entries, `steep`, `hole`, `above`, steep heights,
  minimum surface, secondary noise and column cell are evidence only. An
  earlier call outside the fixture, or a refused call, may have filled those
  caches in the original.
- **Heights view** (`replay.tsv` column `heights`): `final` uses `groups.tsv`
  `final_world_surface_wg` (the centre chunk after genuine `buildSurface`;
  controlled rows never write). `steep-probe` uses `steep-heights.tsv` for that
  group (the synthetic stone chunk of the steep probe, copied from the `y` of
  the raw `steep/lx,lz/top` rows). `validate` checks every recorded Paper
  `steep_heights` cell against the declared view: 838 G, 2107 C (`final`) and
  584 C (`steep-probe`), all equal.

Counts (round 3; generator, `validate` route 2 and a third route in
`round3/delivery/freeze.py` agree, 0 mismatches):

| Tier | `full` | `partial` | `refused` | Total |
| --- | --- | --- | --- | --- |
| G | 2490 | 4230 | 0 | 6720 |
| C | 5478 | 24626 | 2065 | 32169 |
| Total | 7968 | 28856 | 2065 | 38889 |

- The 2065 refused rows are all C rows: 2016 `end/` and 43 `gradient/` rows
  refused as `BlockOutsideGenerationHeight`, and 6 `extreme/` rows refused as
  `InvalidStoneDepth`. No G row lies outside a P4 domain.
- Changes against round 2 (C 5486 / 26683): 6 `full` → `refused`,
  2059 `partial` → `refused`, and 2 `full` → `partial` (ordinals 8725 and 8726
  of the run starting at 8721).
- Every `run_start` and `heights` cell is unchanged.
- The two D rows are diagnostics outside the replay.

## 7. Result type and typed refusals

`SurfaceRuleErrorKind` (the error carries an optional block, node, limit and
requested/maximum counts, no message allocation):

- `InvalidBudget`
- `UnsupportedSettings` (policy P3)
- `UnsupportedNumericProfile` (policy P1)
- `UnsupportedFlatBedrock` (policy P2)
- `MalformedDefinition`
- `UnknownCodecType`
- `InvalidBlockState`
- `UnknownBiome`
- `UnknownNoise`
- `InvalidAnchor`
- `NativeLimit`
- `InconsistentResolver(ResolverMismatch)` with `ResolverMismatch::{Seed,
  Settings, Source, Geometry, ZoomSemantics}`. `Source` and `ZoomSemantics`
  are policy P6 refusals; `Seed`, `Settings` and `Geometry` are consistency
  refusals (R2-H5, §11)
- `ColumnOutsideCentre`
- `NoActiveColumn`
- `BlockOutsideGenerationHeight` (policy P4)
- `InvalidStoneDepth` (policy P4)
- `InvalidWaterHeight` (policy P4)
- `WorkBudgetExceeded`
- `Biome(StoredBiomeErrorKind)`
- `Temperature(BiomeTemperatureErrorKind)`
- `Noise`
- `Poisoned`

Each kind's rustdoc states whether it is a FerrumC policy, a FerrumC limit or
an input/consistency refusal. Tests group them accordingly (review A8).

Refusal order:

1. **Construction:** budget, settings admission, then source limits before
   materialization, codec shape, references (blocks, then biomes, noises,
   random names), census limits.
2. **Bind:** numeric profile (P1), flat-bedrock mode (P2), then the work
   requirement, all before any sampler is built.
3. **Session (`new`):** resolver identity in the order of
   `StoredBiomeIdentity`, then zoom semantics, then centre:
   - seed equal to the binding seed (`Seed`);
   - settings equal to the template resource (`Settings`);
   - source equal to
     `BiomeSourceIdentity::MultiNoise(BiomeParametersResourceId::from_name("minecraft:overworld"))`
     (`Source`, policy P6);
   - geometry with `min_y() == -64` and `height() == 384`, equal to the bound
     settings' `noise.min_y`/`noise.height` (`Geometry`, consistency);
   - `BiomeZoomSemantics::Paper…` (`ZoomSemantics`, policy P6);
   - the centre domain.
4. **Every session method on a poisoned session:** `Poisoned`, before any
   other check.
5. **`begin_column`:** column domain (`ColumnOutsideCentre`), then work
   (`WorkBudgetExceeded` when the remaining allowance is below 2; §8).
6. **`evaluate`:** active column; `y`; stone depths (above, then below);
   water; pre-checked per-call work (§8). After that, evaluation in the
   original lazy order.
7. **`min_surface_level`:** active column (`NoActiveColumn`), then work, only
   when the minimum is not yet cached (`WorkBudgetExceeded` when the remaining
   allowance is below 1; §8). **`surface_depth`:** active column
   (`NoActiveColumn`).

Every refusal in steps 4–7 happens before any mutation. The session is exactly
as before the call: active column, surface depth, every per-column cache and
`work_used` (§8, continuation rules).

A defensive evaluation error (biome, temperature or noise; unreachable for
admitted inputs) poisons the session. No default block, Plains, nearest chunk
or fallback temperature is ever substituted. Failed construction publishes
nothing.

FerrumC policy, not original behaviour (§11):

- P1: the Paper-only numerical profile;
- P2: only `PaperFlatBedrock::Disabled`;
- P3: admission of only the three overworld settings;
- P4: the `y`, stone-depth and water domains;
- P5: the per-call work pre-check and poisoning;
- P6: the resolver profile, i.e. the overworld multinoise source and Paper
  zoom semantics.

Retained oracle evidence shows the original evaluating outside the P4 domains
without exception. All of it is in C rows, none in D rows (corrected in round
3, R2-B1/R2-H2):

- `end/` rows at `y = i32::MIN, -65, 320, i32::MAX`;
- `gradient/` rows at `y = -65, -66`;
- `extreme/` rows with stone depth `i32::MIN/MAX/0/-1`.

The fixture keeps 2065 of these raw rows, all as `refused` replay rows (§6).
Their original values stay evidence and are never expectations. The `extreme/` rows with water
`i32::MAX` and `i32::MIN + 1` are inside the water domain (admitted `Some`).
They are `partial` rows, because their run starts with refused stone-depth
rows.

## 8. Capacities, work and allocation

Caps (overworld in parentheses):

| Item | Cap (overworld) |
| --- | --- |
| nodes | 512 (428) |
| depth | 32 (13) |
| sequence children | 64 (12) |
| biome references | 256 (53) |
| biomes per condition | 16 (3) |
| distinct noises | 16 (9) |
| random names | 8 (2) |
| distinct result states | 128 (24) |
| retained template | 16 KiB (design cap) |
| retained seeded binding | 32 KiB (design cap; ≈ 15680 bytes `ImprovedNoise` + 9×128 bytes `NormalNoise` + octave vectors + 768 bytes bands) |
| session | 4 KiB, no heap (design cap) |

The retained figures are design caps. The runtime measures the exact retained
bytes of template, binding and session, pins them as exact test constants and
tests `NativeLimit` at N-1/N/N+1 of each cap. With 428 nodes, 16 KiB is tight
once a node exceeds 38 bytes. Raising a cap is a Root decision, never a silent
change (review A4).

**Budget domains (review A3).** Each budget's `new(u32)` admits a closed range
and refuses everything else as `InvalidBudget`:

| Budget | Admitted | Domain tests |
| --- | --- | --- |
| `SurfaceRuleBuildBudget` | `1..=262_144` | 0 refused, 1 admitted, 262_144 admitted, 262_145 refused |
| `SurfaceRuleBindBudget` | `1..=65_536` | 0 refused, 1 admitted, 65_536 admitted, 65_537 refused |
| `SurfaceRuleWorkBudget` | `1..=67_108_864` | 0 refused, 1 admitted, 67_108_864 admitted, 67_108_865 refused |

An admitted budget below the requirement is refused as `WorkBudgetExceeded`
before any mutation.

**Compile census** = definition bytes + JSON nodes + compiled nodes +
resolved references = 129,737 + 3,411 + 428 + 202 = **133,778**. The 202
references are 100 block rules + 53 biome names + 33 noise references + 2
random names + 14 anchors (2×2 gradient + 10 `y_above`). Tests: 133,777 /
133,778 / 133,779.

**Bind census** = octave slots (58) + 259 per `ImprovedNoise` for the Paper
basis (56 × 259 = 14,504, the existing `SampleWork` charges) + 192 band slots +
3 factories = **14,757**. Tests: 14,756 / 14,757 / 14,758.

**Per call** (`evaluate`): worst case = rule nodes (287) + condition nodes
(141) + noise samples (33) + secondary (1) + bandlands (1) + gradient draws (2)
+ biome (1) + temperature (1) = **467**.

**Accounting rule (review A3; insufficient allowance settled in round 3,
R2-M1).** "Remaining" is the session allowance minus `work_used()`. Every check
below runs before any mutation.

- `begin_column(SurfaceColumnPos) -> Result<(), SurfaceRuleError>` costs
  2 units (surface noise and draw). After `Poisoned` and the column domain, it
  refuses `WorkBudgetExceeded` when fewer than 2 units remain. A refusal
  changes nothing: an earlier active column stays active with its surface
  depth and caches, and `work_used` is unchanged. The call has no effect, as if
  it had not been made. On success it replaces the active column, resets every
  per-column cache, computes the surface depth and charges 2.
- `min_surface_level(&mut self) -> Result<i32, SurfaceRuleError>` checks
  `Poisoned`, then `NoActiveColumn`. A cached minimum is returned for 0 units
  and never refuses for work. An uncached minimum refuses
  `WorkBudgetExceeded` when fewer than 1 unit remains; the minimum stays
  uncached. Otherwise it computes and caches the minimum and charges 1.
- Computing the column's minimum surface costs 1 unit, exactly once per
  column. It is charged to whichever call computes it first: `evaluate`
  (through `above_preliminary_surface`) or `min_surface_level()`. A cached
  minimum costs nothing.
- `surface_depth(&self) -> Result<i32, SurfaceRuleError>` checks `Poisoned`,
  then `NoActiveColumn`, and costs 0. `work_used(&self) -> u32` is infallible.
- Before mutating anything, `evaluate` refuses when the remaining allowance
  is below `467 + m`, where `m = 1` iff the column's minimum is not yet
  cached. It then charges the units actually used. P4 refusals come before
  this check and charge nothing.

**Continuation after a refusal (R2-M1).**

- After a P4 refusal, `ColumnOutsideCentre` or `NoActiveColumn` the session
  stays usable. The refused call produced nothing, so the caller continues
  with the next block or begins a valid column.
- `WorkBudgetExceeded` from any method does not poison. The allowance never
  grows within a session, however, so the current column cannot be completed
  in that session. That column counts as **not evaluated**. The caller
  discards every result of that column from this session and either refuses
  the chunk or evaluates the column again in a **new session** with a
  sufficient `SurfaceRuleWorkBudget`, starting with `begin_column` and the
  column's first block, from the same heights state that column started with.
- Resuming a column in the middle, carrying cache values from one session to
  another, or combining results of an interrupted column with a restart is
  not allowed: no half cache. Columns completed earlier in the old session
  stay valid, because no per-column state crosses columns.
- `Poisoned` is terminal; only a new session continues.

Session allowance: a full chunk needs at most
256 × (2 + 1 + 384 × 467) = 45,908,736 units; exactly that allowance
suffices.

**Work-boundary test matrix** (N-1/N/N+1 around each charge; "remaining"
before the call):

| Call | Precondition | Remaining | Expected |
| --- | --- | --- | --- |
| `begin_column` | valid column | 1 | `WorkBudgetExceeded`; no effect (earlier column, caches, `work_used` unchanged) |
| `begin_column` | valid column | 2 | `Ok`; remaining 0 |
| `begin_column` | valid column | 3 | `Ok`; remaining 1 |
| `begin_column` | column outside centre | 1 | `ColumnOutsideCentre` (domain before work); no effect |
| `min_surface_level` | no active column | 0 | `NoActiveColumn` |
| `min_surface_level` | minimum uncached | 0 | `WorkBudgetExceeded`; minimum stays uncached (a later `evaluate` still needs `467 + 1`) |
| `min_surface_level` | minimum uncached | 1 | `Ok(min)`; remaining 0 |
| `min_surface_level` | minimum uncached | 2 | `Ok(min)`; remaining 1 |
| `min_surface_level` | minimum cached | 0 | `Ok(min)`; charges 0 |
| `surface_depth` | no active column / active | any | `NoActiveColumn` / `Ok`; charges 0 |
| `evaluate` | minimum cached | 466 / 467 / 468 | `WorkBudgetExceeded` (no effect) / `Ok` / `Ok` |
| `evaluate` | minimum uncached | 467 / 468 / 469 | `WorkBudgetExceeded` (no effect) / `Ok` / `Ok` |
| `evaluate` | input outside a P4 domain | 0 | the P4 kind, not `WorkBudgetExceeded`; no effect |
| any session method | poisoned session | any | `Poisoned`; no effect |
| continuation | `WorkBudgetExceeded` in the middle of a column | — | a new session that re-evaluates the column from its first block equals an uninterrupted evaluation (outcomes, trace, `work_used` of the column) |

Size boundaries are exercised by private `cfg(test)` compiles of synthetic
JSON at 511/512/513 nodes, depth 31/32/33, and so on. These are separate from
original-behaviour tests.

`begin_column`, `evaluate` and `min_surface_level` make zero heap
allocations. This is measured in a separate cold Debug/Release process
following the .7/.8 model, with a positive 4096-byte control. Compile and bind
may allocate; that is counted, not claimed to be free.

The runtime makes no TPS, RSS or speed claims.

## 9. API candidate and runtime ownership

One new module `surface_rules` in `ferrumc-world` (private fields, rustdoc on
everything):

- `SurfaceRuleBuildBudget`, `SurfaceRuleBindBudget` and
  `SurfaceRuleWorkBudget`, each with `new(u32) -> Result` (domains §8);
- `SurfaceRuleTemplate::compile_builtin(NoiseSettingsResourceId, budget)`;
- `SeededSurfaceRules::bind(...)` (§4);
- `SurfaceNumericProfile::{PaperBuild60, Vanilla}`; only `PaperBuild60` is
  admitted (P1);
- `PaperFlatBedrock::{Disabled, Enabled}`; only `Disabled` is admitted (P2);
- `SurfaceRuleSession<'a>::new(&'a SeededSurfaceRules, &'a SurfaceBiomeResolver<'a>, &'a BuiltinBiomeTemperatures, PreliminarySurfaceCorners, SurfaceRuleWorkBudget)`;
- `begin_column(SurfaceColumnPos) -> Result<(), SurfaceRuleError>`,
  `surface_depth() -> Result<i32, SurfaceRuleError>`,
  `evaluate(SurfaceBlockInput, &WorldSurfaceHeights) -> Result<SurfaceRuleOutcome, SurfaceRuleError>`,
  `min_surface_level() -> Result<i32, SurfaceRuleError>` (for the later
  frozen-ocean driver) and `work_used() -> u32`; refusal order and cost in §7
  and §8;
- `SurfaceRuleOutcome::{State(BlockStateId), NoRule}`.

The module docs state the profile: Paper build 60 numerics with Paper zoom
semantics and the Paper default flat-bedrock mode.

Ownership and visibility (review A7):

- No Registry change, no dependency, no change to GEN-04/.5/.6/.7/.8.
- Visibility needs: `NormalNoise::paper_build60_basis` and `SampleWork` are
  already `pub(crate)`. The positional root uses public APIs. Registration
  in `lib.rs` (`pub mod surface_rules` plus module docs) needs explicit
  authorization, as with A1 in GEN-05.8.
- Fixture target path:
  `server/crates/ferrumc-world/tests/fixtures/surface-rules-original-1.21.8/`
  (the 14 files of §10, unchanged). Reference scripts (oracle helper, runner,
  guards, projection, fixture generator) go under
  `server/crates/ferrumc-world/tests/reference/surface-rules/`, next to the
  existing `biome-zoom`/`biome-selection` references.
- `INVARIANTS.md` and `README.md` get their own surface-rules section. It must
  record that the surface binding becomes a second consumer of the Paper
  basis next to the NoiseChunk session: today `INVARIANTS.md` says only that
  Paper Build60 session copies Improved offsets through f32.
- The carried GEN-05.8 runtime condition **R-A1** (pin the two-phase halo
  refusal order of `stored_biome`) belongs in the same World ticket. That means
  a test in `stored_biome/tests` (review A6).

## 10. Oracle protocol and reproducibility

Pins and guards:

- `oracle/prepare.py` declares inputs, cases, mappings and library origins and
  writes `oracle/input-pins.json`: 2061 inputs, SHA-256 `fdfb8b2492053dc6544473f1493591e5a5c20234d81d31a61b19fb7c41ee67cf`. It covers
  the whole archive, artefacts, JDK, mappings, sources, bytecode and design.
  Round 2 changed none of these inputs; the oracle helper and runner are
  unchanged.
- `oracle/run_oracle.py OUT` refuses:
  - the five Java environment variables;
  - helper-package shadows in the original JARs;
  - a noncanonical helper;
  - byte or membership drift, checked before and after;
  - nonfresh or out-of-area output.

  It verifies the inventory of the freshly compiled helper classes and checks
  CodeSources in the JVM.
- Timestamped logs are listed only in `diagnostics-manifest.json`.
- `oracle/check_guards.py OUT REF` exercises 16 guards in the caller's Python
  mode with explicit `require`:
  - 5 environment guards, through the real entrypoint;
  - canonical source, digest, shadow, fresh output and write scope;
  - JDK, archive and design membership;
  - 3 real-JVM CodeSource mismatches, refused before any sample.

  The normal and `-O` `guards.json` differ only in
  `entrypoint_python_optimize`.

Source review and differential evidence are labelled separately throughout:
`sources/`, `bytecode/` and §2.1 versus `final-*`, `round2/oracle-*`,
`projection/` and §2.2. Policy-refusal and malformed-input tests are not
oracle rows.

**Compact fixture (round 3).**
`round3/fixture/surface_fixture.py generate RAW OUT` writes
`round3/fixture/surface-rules-original-1.21.8/` (14 files, about 5.3 MB) from
the unchanged round-2 raw output `round2/oracle-normal`:

- `tree.tsv` (Vanilla JAR = bootstrap = registry form);
- `tree-paper.tsv` and `paper-tree-substitution.json` (Paper JAR form and the
  single node-2 difference; A2);
- `groups.tsv`: corners, final heights, bands, anchors;
- `arrays.tsv`: SHA-256 per Paper/Vanilla stored array, in the declared
  history schedule;
- `columns.tsv`: genuine column noise and draw bits;
- `rows.tsv`: 38891 of 765138 Paper rows, **without a flat-bedrock column**;
- `replay.tsv`: per row `run_start`, replay class (`full`, `partial` or
  `refused`), heights view and the expected P4 refusal kind (`-` when
  admitted) (A1, R2-B1);
- `steep-heights.tsv`: the steep-probe heights view of the three probe groups
  (A1);
- tables for states, biomes, patterns and bands;
- `manifest.json`. It marks the raw flat-bedrock cells as „ungültig, nicht
  maßgeblich (fabrizierter Kontext, Review B1)“. It declares the replay rule
  with its three classes and the P4 refusal order, and it carries the class
  and refusal counts.

Rows are selected by the unchanged round-1 rule:

- 32 rows per (node, outcome) and per shared-condition outcome;
- every Vanilla≠Paper result, surface-depth or biome difference;
- every end/steep/extreme/hole-column row;
- every D row;
- a SHA-256 sample (first byte < 8);
- rows whose invalid round-1 flat cell differs. This criterion only picks
  genuine default-mode rows and is kept so that every byte outside the flat
  cells stays stable; the flat cells themselves are not projected. 2390 C rows
  (`y ∈ [-63, 10]`) are in the fixture only through this picker (R2-H3; the
  manifest counts them as `kept_only_by_invalid_flat_picker`). Their
  expectations come from the genuine default context; no expectation comes
  from flat data.

Projection replaces names by table positions only. Paper columns are the
expectations for `PaperFlatBedrock::Disabled`; there is no flat-bedrock
expectation. Vanilla cells are discriminating evidence only (A3 lesson).

`validate RAW FIX` regenerates byte-for-byte. It then independently expands
every fixture row back to the raw Paper and Vanilla tokens and compares them
(normal and `-O`: 38891 rows equal). It also pins the cells the fixture does
not carry (H8): raw exception `none` in every kept G/C row, cross-tree `=` in
every C row (G rows are not cross-evaluated), and the biome memo either unset
or equal to the direct biome. Finally it re-derives `replay.tsv` along a
second route and checks every recorded steep read against the declared
heights view. The second route uses maximal raw blocks of equal tier, group and
`update_xz`. It takes the P4 kind from the projected `rows.tsv` tokens (`y`,
stone depths, water `-` = `None`), not from the raw row the generator reads.
It recomputes `full` over a kept *and* admitted prefix and compares class and
refusal kind in every row (normal and `-O`: 38889 rows, classes as in §6).

**Byte stability against round 1.** Of round 1's 10 fixture files, eight are
byte-identical (`tree`, `groups`, `arrays`, `columns`, `states`, `biomes`,
`patterns`, `bands`). `rows.tsv` equals round 1's file with exactly the `flat`
column (field 19) removed from every line. `manifest.json` changed. Four files
are new. `round2/delivery/round1-diff.json` records the comparison.

**Byte stability against round 2** (`round3/delivery/round2-diff.json`):

- 12 of 14 fixture files are byte-identical, including `rows.tsv`, `tree*.tsv`,
  `groups.tsv`, `columns.tsv` and `steep-heights.tsv`.
- `replay.tsv` keeps all 38889 lines with unchanged `ordinal`, `run_start` and
  `heights`. It gains the column `refusal`, and the class changes in 2067 lines
  (§6).
- In `manifest.json` only `counts`, `files` (only the `replay.tsv` digest),
  `p4_refusal_order`, `replay`, `round` and `scope` change. `invalid_evidence`,
  `paper_tree`, `raw_sha256`, `selection`, `source_manifest_sha256` and
  `validate_invariants` are unchanged.
- No file was added or removed.
- The generator diff is `round3/delivery/surface_fixture.diff`.
- The raw oracle outputs are the frozen round-2 bytes (29 differential files,
  manifest `15e4dbff…`).

The runtime regenerates the Paper biome arrays with real GEN-05.5 sessions:
cold per group, nine chunks row-major. It compares the digests, then replays
the fixture rows through the real resolver under the replay rule of §6.
Separately it checks node-wise noise and draw bits, steep heights, minimum
surface and clay bands in Debug and Release, and makes the direct
`clay_bands_offset` bit comparison (§13, A5).

## 11. Policy decisions (FerrumC, not original behaviour) — Root decisions of rounds 2 and 3

- **P1 — Paper-only numerical profile. Accepted.** The upstream density,
  material and biome producers are Paper build 60
  (`SeededRouter::fresh_paper_noise_chunk`; the stored arrays of GEN-05.5 are
  Paper). A Vanilla surface on Paper terrain would be a hybrid that no
  original produces. A Vanilla request is refused typed
  (`UnsupportedNumericProfile` at bind), never computed silently as Paper.
  Vanilla stays oracle evidence.
- **P2 — Flat bedrock. Decided: only `Disabled` ships.** `Disabled` (Paper
  default) equals the Vanilla gradient in every cross-tree comparison.
  `Enabled` is refused typed (`UnsupportedFlatBedrock`) as a FerrumC policy
  until a separate world-configuration ticket exists. The enum keeps both
  variants so that ticket does not change the signature. There is **no valid
  evidence for `Enabled`** (B1, §2.2); that ticket needs fresh evidence from a
  corrected harness.
- **P3 — Admission of the three overworld settings only. Accepted.**
  `tree-variants.tsv` shows canonical equality in both modes.
- **P4 — Block domains (`y`, stone depth, water). Accepted.** The original
  evaluates outside them; the fixture classifies those 2065 rows as `refused`
  and expects only the typed refusal, never original equality (§6, §7; round
  3, R2-B1).
- **P5 — Session work pre-check and poison on defensive errors. Accepted with
  condition A3**, settled in §8: budget domains, the accounting rule, and the
  insufficient-allowance behaviour and continuation of round 3 (R2-M1).
- **P6 — Resolver profile. Accepted as a consistency rule.** Two parts:
  - **Zoom semantics:** the resolver must use Paper zoom semantics; mismatch →
    `InconsistentResolver(ZoomSemantics)`. This is not a numerical necessity:
    both zoom semantics produce identical results here (0 zoom-caused
    differences; GEN-05.8 H-2). The rule keeps the session on one consistent
    Paper profile.
  - **Source (round 3, R2-H5):** the resolver identity's source must be
    `MultiNoise(minecraft:overworld)`; mismatch →
    `InconsistentResolver(Source)`. The three admitted settings are the
    `normal`, `large_biomes` and `amplified` world presets. All three use the
    overworld multinoise parameter list, and the oracle drove exactly these
    presets' genuine overworld generators (78 / 1 / 1 groups,
    `SurfaceRulesOracle.java:496-503`). The original would also evaluate the
    rules over a fixed or other source. Refusing that is FerrumC scope, so it
    is policy.

  The resolver **geometry** check (`InconsistentResolver(Geometry)`, round 3)
  is a consistency refusal, not policy. The original resolves the generation
  context from the chunk's level height (`minY = max(level, settings)`,
  `genDepth = min(level, settings)`, §4). Its biome accessor clamps quart `y`
  to that height. The oracle built every chunk with
  `LevelHeightAccessor.create(-64, 384)` (`SurfaceRulesOracle.java:447`), and
  all 80 groups record `min_gen_y = -64`, `gen_depth = 384`. A resolver geometry
  other than the settings' `(-64, 384)` would therefore change anchors and biome
  clamping in a way this contract does not cover.

## 12. Open points and limitations

- **B1 is open for the flat-bedrock `Enabled` mode.** The round-1 flat probe
  ran on a fabricated column state (§2.2). Its cells stay in the frozen raw
  outputs, marked invalid, and are excluded from fixture, expectations and
  claims. A future world-configuration ticket would need the harness fixed
  (per-probe column tracking) and fresh evidence. This contract does not
  depend on it, because `Enabled` is refused.
- **Coverage is complete in both originals.** Every lazy condition node
  shows both outcomes, and every finite noise threshold is observed below,
  within and above (`projection/coverage.json`). One declared band search
  (badlands, `surface` > 0.92) found no column within its radius
  (`band-absent`). Node 68's "above max" side is covered through the
  wooded-badlands band group instead (`noise_threshold_sides` for nodes
  22/68: {"22": {"above_max": 37, "below_min": 432, "within": 204}, "68": {"above_max": 38, "below_min": 1526, "within": 234}}).
- **Not executed:**
  - no `WorldGenRegion`, `ServerLevel` tick or carver;
  - the column driver was only driven through genuine `buildSurface` with a
    recording proxy;
  - the Vanilla stateful temperature cache is not separately stressed. Its
    cross-sea-level stale reads cannot occur within one overworld (sea level
    63, positions within packed-key range); GEN-05.7 retains the diagnostics.
- **`clay_bands_offset`** has no basis evidence. The bandlands outcomes cannot
  discriminate the Paper from the Vanilla basis (0 / 3393 badlands columns
  differ in the band offset, although the bits differ in 3393 / 3393). The
  direct bit comparison is a mandatory runtime condition (§2.4, A5).
- **Vanilla history environment:** single worker (`-Dmax.bg.threads=1`),
  cold RTree history per group (GEN-05.8 A5).
- **Raw outputs** are about 474.3 MB (decimal) = 452 MiB per run: round 1's
  `final-normal` is 474,301,331 bytes, round 2's `oracle-normal` 474,300,753.
  Only the timestamped logs differ; the 29 differential files total
  473,872,928 bytes in both. Only the compact fixture is proposed for commit.
- **Design figures:** layout and census figures are design values; retained
  bytes, scratch and allocation freedom must be measured in the runtime (A4).
- **Non-authoritative, excluded from the freeze:** `scratch/`,
  `registry-probe/{archive,target}`, `registry-probe/dev2-probe/*` and
  `registry-probe/warm-debug.log` (round-1 development leftovers, H7),
  `round2/scratch/` and `round2/paper-basis-probe/{archive,target}`,
  `round3/scratch/` (trial generation) and `round3/clay-probe/{archive,target}`.
- **Root review:** Root reproduces and reviews contract and oracle
  independently before any runtime authorization. No Survival, plugin, TPS,
  RSS or activation claim follows.

## 13. Auflagen für das Runtime-Ticket (review A1–A8)

Conditions for the later runtime ticket. Each cites the review section
„Auflagen fürs Runtime-Ticket“ of `GEN-05_9_SURFACE_RULES_REVIEW.md`. Round-3
additions cite the findings of `GEN-05_9_SURFACE_RULES_REVIEW_ROUND2.md`
(R2-B1, R2-M1, R2-H1, R2-H5).

- **A1 — Replay semantics of the fixture** (review A1, R2-B1). *Settled in
  this contract* (§6 replay rule, §10 `replay.tsv`, `steep-heights.tsv`).
  Condition: the runtime replays the three classes as follows:
  - `full` rows by column run with full trace equality (only runs whose prefix
    is completely kept and admitted);
  - `partial` rows alone with the per-block subset;
  - `refused` rows alone, expecting exactly the `refusal` kind with no outcome
    and an unchanged `cfg(test)` state projection.

  It uses the declared heights view. It must not add rows, relax the classes,
  or compare a refused row's original values.
- **A2 — Paper tree form committed mechanically** (review A2). *Settled*:
  `tree-paper.tsv` and `paper-tree-substitution.json` are in the fixture (§3).
  Condition: the post-substitution check compares against these files, never
  against a hand-written form.
- **A3 — Budget domains and work accounting** (review A3). *Settled in §8*.
  Condition: tests at both domain ends of all three budgets (`InvalidBudget`),
  the census N-1/N/N+1 tests, the `467 + m` pre-check tests with and
  without a cached minimum, and every row of the round-3 work-boundary matrix
  of §8 (R2-M1). That matrix covers `begin_column` at remaining 1/2/3,
  `min_surface_level` at 0/1/2 uncached and 0 cached, domain before work,
  P4 before work, `Poisoned` first, the no-effect property of each refusal,
  and the restart-in-a-new-session equivalence.
- **A4 — Retained capacities pinned exactly** (review A4). Remains a
  condition: measure template, binding and session bytes, pin them as exact
  constants, test `NativeLimit` at N-1/N/N+1. A cap change is a Root decision.
- **A5 — Paper basis of all nine noises bit-exact** (review A5). *Design
  evidence now* (§2.4): 8 of 9 noises bit-exact in the existing Paper basis,
  node-wise for all 33 threshold nodes. Condition: the runtime repeats the
  node-wise and noise-wise bit comparisons against the fixture in its own
  tests (Debug and Release). **`clay_bands_offset` (round 3, R2-H1):** the
  bandlands outcomes do not discriminate the basis (§2.4), so a **direct bit
  comparison is mandatory**. It compares the runtime's
  `clay_bands_offset.sample(x, 0, z)` bits against genuine Paper
  `SurfaceSystem.clayBandsOffsetNoise.getValue(x, 0, z)` values, with the
  Vanilla basis as the discriminating control (bits differ in 3393 / 3393
  badlands columns). The frozen oracle has no such values. The runtime ticket
  therefore needs a Root-authorized, pinned helper extension. That extension
  samples this noise in genuine original contexts (at least the 3393 badlands
  columns, both originals) and commits the bits as a mechanical fixture
  projection. Without this comparison A5 is not met.
- **A6 — Carry R-A1** (review A6): a test in `stored_biome/tests` pinning the
  two-phase halo refusal order, equivalent to R1 of the GEN-05.8 runtime
  review.
- **A7 — Ownership and documentation** (review A7): `pub mod surface_rules`
  in `lib.rs` only with explicit authorization; fixture and reference paths
  of §9; a surface-rules section in `INVARIANTS.md`/`README.md` naming the
  second Paper-basis consumer.
- **A8 — Distinguishable policy refusals and steep timing** (review A8):
  `InconsistentResolver` carries `ResolverMismatch::{Seed, Settings, Source,
  Geometry, ZoomSemantics}`, tested in the order of §7 with one mismatching
  field at a time. The `Source` tests use `Fixed` and `End`. The `Geometry`
  tests use a different `min_y` and a different `height` (R2-H5).
  `UnsupportedNumericProfile` and `UnsupportedFlatBedrock` are their own
  policy kinds (§7). A controlled test changes the heights view
  after a column's first steep test, and the result must keep the first
  test's value (§5, H6). A second controlled test puts a refused call before
  the first steep test. The first admitted steep test then fixes the heights
  view (§5).

## 14. Fresh runs and freeze (rounds 2 and 3)

### Round 3 (no oracle run)

Round 3 ran, all into new paths under `round3/`. It consumes the frozen round-2
raw outputs unchanged:

```text
python3    oracle/check_guards.py round3/guards-normal    round2/oracle-normal
python3 -O oracle/check_guards.py round3/guards-optimized round2/oracle-normal
python3    round3/fixture/surface_fixture.py generate round2/oracle-normal    round3/fixture/surface-rules-original-1.21.8
python3 -O round3/fixture/surface_fixture.py generate round2/oracle-normal    round3/fixture/generate-optimized
python3    round3/fixture/surface_fixture.py generate round2/oracle-optimized round3/fixture/generate-from-oracle-optimized
python3    round3/fixture/surface_fixture.py validate round2/oracle-normal    round3/fixture/surface-rules-original-1.21.8
python3 -O round3/fixture/surface_fixture.py validate round2/oracle-normal    round3/fixture/surface-rules-original-1.21.8
python3    round3/clay-probe/run_probe.py round2/oracle-normal round3/clay-probe/final
python3    round3/forbidden_claims_check.py > round3/delivery/forbidden-claims.json
python3    round3/delivery/freeze.py
```

(`round3/run_fixture_round3.sh` holds the five fixture commands.) Results:

- **Guards:** 16/16 refused in both modes. Normal and `-O` differ only in
  `entrypoint_python_optimize`. Both `guards.json` are byte-identical to
  round 2.
- **Fixture:** the normal and `-O` generations and the generation from
  `round2/oracle-optimized` are byte-identical in all 14 files. Validation
  output is identical in both modes:
  - 38891 expanded rows equal the raw rows, and the H8 invariants hold;
  - 38889 replay rows are re-derived with classes G 2490 / 4230 / 0 and
    C 5478 / 24626 / 2065 (`full` / `partial` / `refused`);
  - 3529 steep reads are consistent.
- **Third replay route** (`freeze.py`, independent of the generator code):
  0 mismatches in run start, class or refusal kind. It names the run (start
  8721) whose `full` rows 8725/8726 became `partial`.
- **Clay probe:** Debug = Release, figures as in §2.4. The private copy
  differs from the archive only by the module and the registration lines;
  no probe module is in the working tree.
- **Forbidden claims:** the archive's scanner (`markdown_blocks` + `scan`,
  `force_plugin` false and true) is applied to this document and the work
  order (`round3/delivery/forbidden-claims.json`).
- **`round3/delivery/freeze.py`** re-verifies:
  - the oracle input pins;
  - both preserved candidates;
  - every round-2 freeze entry (372, which include round 1's);
  - the consumed raw outputs;
  - guards, fixture, the third route and the clay probe.

  It writes `round3/delivery/final-compare.json`,
  `round3/delivery/round2-diff.json`, `round3/delivery/surface_fixture.diff`
  and the whole-file `round3/delivery/freeze-manifest.json`. That manifest
  contains every round-2 entry unchanged and keeps `invalid_evidence`.
- The freeze-manifest SHA-256 and this document's SHA-256 are reported to Root
  separately.

### Round 2

Round 2 ran, all into new paths:

```text
python3    oracle/run_oracle.py round2/oracle-normal
python3 -O oracle/run_oracle.py round2/oracle-optimized
python3    oracle/check_guards.py round2/guards-normal    round2/oracle-normal
python3 -O oracle/check_guards.py round2/guards-optimized round2/oracle-normal
python3    round2/fixture/surface_fixture.py generate round2/oracle-normal round2/fixture/surface-rules-original-1.21.8
python3 -O round2/fixture/surface_fixture.py generate round2/oracle-normal round2/fixture/generate-optimized
python3    round2/fixture/surface_fixture.py validate round2/oracle-normal round2/fixture/surface-rules-original-1.21.8
python3 -O round2/fixture/surface_fixture.py validate round2/oracle-normal round2/fixture/surface-rules-original-1.21.8
python3    round2/paper-basis-probe/run_probe.py final-normal round2/paper-basis-probe/final
python3    round2/forbidden_claims_check.py > round2/delivery/forbidden-claims.json
python3    round2/delivery/freeze.py
```

Results:

- All 29 differential outputs and `manifest.json` of both fresh runs are
  byte-identical to each other and to round 1's `final-normal` and
  `final-optimized` (manifest SHA-256
  `15e4dbff186d8845143f72a0873f28bd35a87cbff4063dfe2e01d1e222969073`). The
  raw rows, including the invalid flat cells, therefore reproduce exactly.
- Guards: 16/16 refused in both modes; normal and `-O` differ only in
  `entrypoint_python_optimize`, and both equal round 1's `guards.json`.
- Fixture: normal and `-O` generation are byte-identical; validation in both
  modes reports 38891 expanded rows, the H8 invariants, 38889 re-derived
  replay rows and 3529 consistent steep reads.
- The Paper-basis probe was run against `final-normal`, whose consumed files
  are byte-identical to `round2/oracle-normal`.
- The archive's forbidden-claims scanner (`scripts/check-forbidden-claims.py`,
  `markdown_blocks` + `scan`, with `force_plugin` false and true) reports 0
  findings for this document and the work order.
- `round2/delivery/freeze.py` re-verifies pins, both fresh runs, guards,
  fixture, round-1 byte diff and probe receipts. It writes
  `round2/delivery/final-compare.json`, `round2/delivery/round1-diff.json` and
  the whole-file `round2/delivery/freeze-manifest.json`. That manifest also
  lists the invalid flat evidence under `invalid_evidence`.
- The freeze-manifest SHA-256 and this document's SHA-256 are reported to Root
  separately.

## 15. Änderungen Runde 2

| Befund / Entscheidung | Änderung | Stelle |
| --- | --- | --- |
| B1 (fabrizierter Flat-Kontext) | Flat-Probe als ungültig/nicht maßgeblich beschrieben; Ergebniszeile ersetzt; die Aussage „backed by genuine evidence“ (Runde 1 Z. 205, 637) gestrichen; B1 als offener Punkt | §2.2, §11 P2, §12 |
| B1 Daten | Rohdaten unverändert; im Fixture-Manifest und im Freeze-Manifest als „ungültig, nicht maßgeblich (fabrizierter Kontext, Review B1)“ markiert | §10, `round2/fixture/…/manifest.json`, `round2/delivery/freeze-manifest.json` |
| B1 Fixture | Flat-Spalte aus `rows.tsv` entfernt; Generator angepasst; Zeilenauswahl unverändert; außerhalb der Flat-Zellen kein Byte geändert | §10, `round2/fixture/surface_fixture.py`, `round2/delivery/round1-diff.json` |
| P1 | angenommen; `SurfaceNumericProfile`, typisierte Verweigerung `UnsupportedNumericProfile` | §4, §7, §9, §11 |
| P2 | nur `Disabled`; `Enabled` → `UnsupportedFlatBedrock`; Enum bleibt | §3, §4, §6, §7, §9, §11 |
| P3, P4 | angenommen | §11 |
| P5 + A3 | angenommen; Budget-Domains, Abrechnungsregel `467 + m` | §8, §11 |
| P6 (H11) | angenommen als Konsistenzregel (beide Zoom-Modi ergebnisgleich); in die Policy-Liste aufgenommen; `ResolverMismatch::ZoomSemantics` | §5, §7, §11 |
| A1 | jetzt gelöst: mechanische Replay-Regel, `replay.tsv`, `steep-heights.tsv`, Zweitweg-Prüfung | §6, §10, §13 |
| A2 | jetzt gelöst: `tree-paper.tsv`, `paper-tree-substitution.json` in der Fixture | §3, §10, §13 |
| A3 | jetzt gelöst im Vertrag; Tests als Auflage | §8, §13 |
| A4 | Auflage | §8, §13 |
| A5 | jetzt Design-Evidenz (8/9 Noises bitgenau, 33/33 Knoten); `clay_bands_offset` offen indirekt; Runtime-Wiederholung als Auflage | §2.4, §4, §13 |
| A6 | Auflage (R-A1) | §9, §13 |
| A7 | Zielpfade, `lib.rs`-Freigabe, INVARIANTS/README-Abschnitt | §9, §13 |
| A8 | unterscheidbare Policy-Verweigerungen; Steep-Zeitpunkt-Kontrolltest | §5, §7, §13 |
| H1 | „36 surface depths“ → 36 Zeilen aus einer Spalte; Treiber Noise-Basis | §2.2 |
| H2 | `tryApply`-Aufrufe aus echtem `buildSurface` (80 Aufrufe pro Modus) | §2.2 |
| H3 | 180 statt 181 geteilte Bootstrap-Positionen (beide Modi nachgezählt) | §3 |
| H4 | 474.301.331 Byte = 474 MB dezimal = 452 MiB | §12 |
| H5 | 18.984 gemeinsame Spalten; 26.556 gemeinsame Zeilen mit Spaltenzelle | §2.2 |
| H6 | Per-Call-View quellbasiert, kein Differentialfall; Kontrolltest | §5, §13 A8 |
| H7 | `registry-probe/dev2-probe/*`, `warm-debug.log` als nicht maßgeblich deklariert und vom Freeze ausgenommen | §12, Freeze-Scope |
| H8 | `validate` prüft Exception, Cross-Trees und Biome-Memo explizit | §10 |
| H9 | Java-`%` begründet; negative Indizes → defensiver `Noise`-Fehler; Test | §6 |
| H10 | §2.3 als reine Vanilla-Basis gekennzeichnet; Paper-Basis in §2.4 | §2.3, §2.4 |
| H11 | P6 in der Policy-Liste | §7, §11 |

## 16. Änderungen Runde 3

Grundlage: Review Runde 2 (`GEN-05_9_SURFACE_RULES_REVIEW_ROUND2.md`,
`6a3ddd8c…`, Assessment `16f74333…`) und die Root-Entscheidungen dazu. Es gab
keinen Oracle-Lauf. Unverändert sind die Oracle-Rohdaten, `rows.tsv`, die
Zeilenauswahl und 12 von 14 Fixture-Dateien (§10). Der Runde-2-Stand liegt
unverändert in `round2-candidate/`.

| Befund / Entscheidung | Änderung | Stelle |
| --- | --- | --- |
| R2-B1 (Replay-Regel gegen P4) | dritte Klasse `refused` mit erwarteter typisierter Policy-Art (Spalte `refusal`: `BlockOutsideGenerationHeight` bzw. `InvalidStoneDepth`); erwartet wird genau diese Verweigerung ohne Teilergebnis und ohne Zustandsänderung, keine Ergebnisgleichheit | §6, §10, `round3/fixture/…/replay.tsv` |
| R2-B1 Folgefehler | `full` nur bei vollständig enthaltenem **und** zulässigem Run-Präfix; eine verweigerte Zeile im Präfix macht die folgenden zulässigen Zeilen derselben Spalte zu `partial` (8725, 8726 im Run ab 8721) | §6, §5 (Steep-Zeitpunkt nach Verweigerung) |
| R2-B1 Validator | `validate` leitet alle drei Klassen und die Verweigerungsart auf dem zweiten Weg aus den projizierten `rows.tsv`-Tokens neu ab; `freeze.py` prüft einen dritten Weg aus den Rohzeilen | §10, §14, `round3/fixture/surface_fixture.py`, `round3/delivery/freeze.py` |
| R2-B1 Zählungen | G 2490 / 4230 / 0, C 5478 / 24626 / 2065 (`full` / `partial` / `refused`); Übergänge gegenüber Runde 2: 6 `full`→`refused`, 2059 `partial`→`refused`, 2 `full`→`partial` | §6, §14, `round3/delivery/round2-diff.json` |
| R2-B1 A1 | Auflage auf drei Klassen umgestellt; Originalwerte verweigerter Zeilen sind nie Erwartung | §13 A1, §11 P4 |
| R2-H2 (§7 Z. 638 „D rows“) | korrigiert: alle Evidenz außerhalb der P4-Domains sind C-Zeilen (`end/`, `gradient/`, `extreme/`); die Wasser-Extreme sind zulässig und `partial` | §7 |
| R2-M1 (Restbudget) | `begin_column` → `Result<(), _>`: nach `Poisoned` und Spalten-Domain `WorkBudgetExceeded` bei Rest < 2, ohne Wirkung; `min_surface_level` → `Result<i32, _>`: `Poisoned`, `NoActiveColumn`, dann `WorkBudgetExceeded` nur ungecacht bei Rest < 1, Minimum bleibt ungecacht; `surface_depth` → `Result<i32, _>` | §7, §8, §9 |
| R2-M1 Fortsetzung | Verweigerungen sind wirkungslos; nach P4/Domain weiterarbeiten; nach `WorkBudgetExceeded` gilt die Spalte als nicht ausgewertet: neue Session mit ausreichendem Budget, `begin_column`, Neustart ab dem ersten Block, kein halber Cache, keine Mischung | §8 |
| R2-M1 Matrix | Work-Grenz-Testmatrix (begin_column 1/2/3, min_surface_level 0/1/2 ungecacht und 0 gecacht, Domain vor Work, P4 vor Work, `Poisoned` zuerst, Neustart-Äquivalenz) | §8, §13 A3 |
| Klarstellung zu R2-M1 | `Poisoned` als erste Prüfung jeder Session-Methode festgeschrieben (bisher nur implizit) | §7 |
| R2-H1 (`clay_bands_offset`) | ehrlich korrigiert: keine Basis-Evidenz, die Bandlands-Ergebnisse unterscheiden Paper und Vanilla nicht (0/3393). Neue Design-Probe `round3/clay-probe/`: Bits unterscheiden sich in 3393/3393 Spalten, der Band-Offset in 0/3393 (max. Δ 7,56e-6, min. Rundungsabstand 1,06e-4). Ein direkter Bitvergleich gegen echte Werte ist ohne Oracle-Lauf nicht möglich; er ist Pflicht-Auflage A5 mit von Root freizugebender Helper-Erweiterung | §2.4, §4, §10, §12, §13 A5 |
| R2-H5 (Resolver-Quelle/Geometrie) | `ResolverMismatch::{Seed, Settings, Source, Geometry, ZoomSemantics}`; Reihenfolge wie `StoredBiomeIdentity`, dann Zoom; `Source` = `MultiNoise(minecraft:overworld)` (Policy P6); `Geometry` = `(-64, 384)` = Settings `noise.min_y/height` (Konsistenz; Beleg `SurfaceRulesOracle.java:447`, 80/80 Gruppen) | §5, §7, §11 P6, §13 A8 |
| R2-H3 | übernommen als Offenlegung: 2390 C-Zeilen nur über den ungültigen Flat-Picker; im Fixture-Manifest gezählt (`kept_only_by_invalid_flat_picker`) | §10 |
| R2-H4 | übernommen: 18.984 Spalten je Gruppe, 18.728 physische (Preset, Seed, x, z), selbst nachgezählt | §2.2 |
| Nicht übernommen | keiner der Hinweise abgelehnt | — |
