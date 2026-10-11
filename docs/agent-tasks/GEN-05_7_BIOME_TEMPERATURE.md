# GEN-05.7 — Original builtin biome temperature for surface rules

Status: concrete design and original constructor/resource census complete;
Root review pending. No FerrumC runtime implementation or temperature fixture
exists yet. GEN-05.6 is integrated as34c8875e and remains unchanged.

## One complete producer

One crate: ferrumc-world; no Registry edit or new dependency. This producer
loads every checked builtin biome's temperature climate metadata and executes
Paper1.21.8-build60's stateless temperature and pure precipitation predicates.
It supplies `SurfaceRules.Context.TemperatureHelperCondition` and the frozen
Ocean iceberg temperature dependency. It does not resolve coordinates into
biomes, mutate snow/ice/water, evaluate lighting/support, run surface rules or
activate world generation.

The common arithmetic is the genuine private `Biome.getHeightAdjustedTemperature`
in both Vanilla1.21.8 and Paper1.21.8-build60. Public Paper `getTemperature`
directly invokes it. Public retained Vanilla `getTemperature` has additional
observable cache state and is deliberately a separate diagnostic, not a second
stateless semantics variant falsely claiming that public contract.

## Proposed typed API

- `BiomeTemperatureBuildBudget::new(u32)` admits1..=1048576 units and rejects
  invalid allowances before any construction. Its value is private.
- `BuiltinBiomeTemperatures::compile(budget)` builds one complete immutable
  checked table and its fixed noise objects; returns a typed error on failure.
  It owns a fixed65-entry climate/checked-ID table and exactly five fixed
  Simplex objects by value, with no heap, mutable cache/global or `WorldSeed`
  parameter. Clone copies this bounded immutable value without allocation.
  Callers may wrap it in their own immutable Arc; that allocation belongs to
  the caller. Query uses bounded binary search of checked IDs, never a numeric
  wire-ID assumption, and allocates nothing.
- `sample(&self, BiomeResourceId, BlockPos, sea_level: i32)` returns
  `Result<BiomeTemperatureSample, BiomeTemperatureError>`. All i32 axes and
  sea-level bits are supported. The checked biome ID selects existing metadata;
  no arbitrary climate, name or JSON constructor becomes public.
- `BiomeTemperatureSample` has private fields and read-only `temperature()`
  (original f32), `warm_enough_to_rain()`, `cold_enough_to_snow()`,
  `should_melt_iceberg_slightly()` and `precipitation()` methods. The last returns
  explicit `BiomePrecipitation::{None,Rain,Snow}` using the actual biome's
  precipitation flag. Raw f32 bits remain observable through the f32 value.
- `BiomeTemperatureError` exposes checked resource/field/offset context through
  methods. A typed `BiomeTemperatureErrorKind` distinguishes invalid allowance,
  source/census limits, malformed JSON, missing/invalid climate field, work
  exhaustion, underlying `NoiseError`, and nonfinite completed
  temperature. `BiomeTemperatureLimit` identifies source bytes, JSON nodes/
  depth/container/string limits, records, retained native bytes and scratch.
  Error context is an optional checked biome ID, static climate field name,
  source offset, optional BlockPos/sea level, plus typed limit count/maximum or
  raw nonfinite bits. It uses no heap message, `anyhow`, silent normalization
  or fallback temperature. No allocation-error variant is invented when the
  concrete implementation has no fallible heap allocation.

The fallible query follows the existing exact Simplex sampler's error boundary;
all admitted i32 inputs are within its current coordinate domain. The largest
absolute scaled input is429496729.6 (Info X/Z*0.2), below the existing
MAX_NOISE_COORDINATE2147483136.0 cap; the original2D skewed floor/sum domain
also stays within signed32 for these coordinates. No error is
silently converted into warm/cold or a substitute biome.

## Actual source behavior and fixed noise construction

Pinned Paper commit `29c8822d90899c89d2689338e81a98f690bcba12`, protocol772.
Read complete Base/Applied `Biome`, complete `PerlinSimplexNoise`, `SimplexNoise`,
`WorldgenRandom`, `LegacyRandomSource`, the `Biome` Source patch and Moonrise
feature hunk. Related full `SurfaceSystem`, `SurfaceRules` and `RandomState`
and relevant `WorldGenerationContext`, `VerticalAnchor`, NoiseGenerator surface
callers and stored biome getter methods were also read.

The noises are independent of the world seed: Temperature uses original Legacy
seed1234 with octaves[0], FrozenTemperature seed3456 with[-2,-1,0], and BiomeInfo
seed2345 with[0]. These require five original Simplex objects. PerlinSimplex's
negative octave order and factor accumulation must remain original; every
sampling call here uses `useNoiseOffsets=false`. Implement only the private
fixed configurations needed by this producer, reusing existing exact Rust
Simplex2D and Legacy RNG. No generic user octave list/seed API is added.

The scratch original helper directly constructs the real PerlinSimplexNoise
with real counted WorldgenRandom/LegacyRandomSource, then calls genuine
`getCount`. Five actual original class CodeSources are checked against each
pinned inner JAR. Results in both original modes:262 draws for1234/[0],786
for3456/[-2,-1,0],262 for2345/[0], total1310 core draws. This is actual original
constructor evidence, not a rewritten RNG estimate or temperature acceptance.
Normal and Python-O receipts live in `.codex-tmp/gen05_7/census/`.

Preserve these arithmetic details:

- FROZEN computes three-octave noise at X/Z*0.05, times7, plus BiomeInfo at
  X/Z*0.2. Only if the sum is strictly below0.3 does it sample BiomeInfo at
  X/Z*0.09; strictly below0.8 returns original f32 constant0.2, otherwise the
  original base temperature.
- The altitude branch computes `sea_level + 17` with signed32 wrap. X/Z are
  converted/divided as f32 by8.0F before widening into the f64 noise call.
  Multiply that noise result by8.0 and round to f32 before height arithmetic.
- Preserve the original f32 expression
  `f - (f1 + Y - i) * 0.05F / 40.0F` left to right. Computing an integer Y-i
  first, keeping f64 intermediates or reassociation changes boundary results.
- Rain threshold is `temperature >= 0.15F`, cold is its negation, iceberg melt
  is strictly `temperature > 0.1F`. No precipitation gives None independently
  of temperature; other biomes give Snow/Rain from the original threshold.

At most six Simplex2D evaluations occur per query (three Frozen, two BiomeInfo,
one altitude). No query recursion, user-sized loop, allocation, cache mutation
or RNG advancement exists. Thus this producer needs no cumulative query budget
or poison state; later surface sessions must meter their own sequence of calls.

## Complete roster and parser/admission census

REG-BIOME has67 full preserved definition records:65 actual Biome definitions
plus2 multinoise parameter-list definitions, `minecraft:nether` and
`minecraft:overworld`, which each contain a preset key and no ClimateSettings.
The latter are checked `BiomeParametersResourceId`, not `BiomeResourceId`, and
must never enter this temperature table. All65 BiomeResourceIds are admitted,
including Nether, End, the_void and pale_garden. There is no Plains default.

The actual versioned original worldgen export SHA256 is
`bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30`.
The exact full JSON slices used by Registry total338695 bytes and14466 nodes,
with maximum6871 bytes,293 nodes, depth5,30 container entries and45 decoded
string bytes per definition. This differs from the original JAR category's
248743 raw bytes: these are separately formatted complete export slices.
`.codex-tmp/gen05_7/census/metadata.json` preserves all65 exact slice hashes,
field tokens, structural metrics and the separately excluded2 presets.

All names below have the `minecraft:` namespace. Except the two stated Frozen
rows, modifier is NONE. These groups enumerate all65 actual definitions:

| Base temperature | Precipitation | Modifier | Complete biome names |
| --- | --- | --- | --- |
| 2.0 | false | NONE | badlands, basalt_deltas, crimson_forest, desert, eroded_badlands, nether_wastes, savanna, savanna_plateau, soul_sand_valley, warped_forest, windswept_savanna, wooded_badlands |
| 0.95 | true | NONE | bamboo_jungle, jungle, sparse_jungle |
| 0.8 | true | NONE | beach, deep_dark, dripstone_caves, mangrove_swamp, plains, sunflower_plains, swamp |
| 0.6 | true | NONE | birch_forest, old_growth_birch_forest |
| 0.5 | true | NONE | cherry_grove, cold_ocean, deep_cold_ocean, deep_lukewarm_ocean, deep_ocean, lukewarm_ocean, lush_caves, meadow, ocean, river, warm_ocean |
| 0.7 | true | NONE | dark_forest, flower_forest, forest, pale_garden |
| 0.5 | true | FROZEN | deep_frozen_ocean |
| 0.5 | false | NONE | end_barrens, end_highlands, end_midlands, small_end_islands, the_end, the_void |
| 0.0 | true | FROZEN | frozen_ocean |
| -0.7 | true | NONE | frozen_peaks, jagged_peaks |
| 0.0 | true | NONE | frozen_river, ice_spikes, snowy_plains |
| -0.2 | true | NONE | grove |
| 0.9 | true | NONE | mushroom_fields |
| 0.3 | true | NONE | old_growth_pine_taiga |
| 0.25 | true | NONE | old_growth_spruce_taiga, taiga |
| 0.05 | true | NONE | snowy_beach |
| -0.3 | true | NONE | snowy_slopes |
| -0.5 | true | NONE | snowy_taiga |
| 1.0 | true | NONE | stony_peaks |
| 0.2 | true | NONE | stony_shore, windswept_forest, windswept_gravelly_hills, windswept_hills |

Metadata decoding uses a private allocation-free token cursor over admitted
static Registry definitions, with the existing World's JSON grammar as its
syntax reference. It does not call Density's String/Vec/BTreeMap DOM parser,
edit that parser, add a serde dependency/RawValue feature or change the lock.
Mandatory base temperature is parsed from its preserved decimal number token
directly to f32 (no intermediate f64); precipitation must be a JSON BOOL.
Only an absent temperature_modifier defaults to NONE. A present modifier must
decode to exactly none or frozen. Invalid climate field types/values fail;
unrelated effects/spawn/generation fields are syntax-validated and skipped.

The cursor consumes the complete input using only JSON whitespace, strict
number/literal grammar and checked UTF-8/string escapes. Unicode surrogate
pairs must be valid. Two fixed256-byte decoded UTF-8 buffers compare keys by
their actual decoded identity, including escaped/unescaped duplicate aliases.
An explicit depth-indexed frame array holds at most33 frames (root depth0
through32), with at most256 borrowed key ranges per object; arrays retain only
their entry count. Duplicate checks compare against previously retained ranges
and need no string allocation. Every skipped value and object key is counted
as a node; root depth is0, matching the original-resource census. Fixed frames,
two buffers, climate candidates and constructor temporaries all count as
simultaneously live native scratch, capped at1MiB. No runtime JSON error carries
an allocated path/string. The checks apply equally to the private test decoder.

Admission caps:65 records;128KiB/source,4096 nodes/source, depth32,256
container entries,256 decoded string bytes;2MiB aggregate source bytes across
all65 distinct resource IDs (equal-content resources are each charged);
8KiB retained native table/noises;1MiB simultaneous parsing/build scratch.
The actual retained layout and allocated capacities must be measured after
implementation; these are proposed caps, not measured performance results.

Construction work is a declared admission census: one unit per source byte,
JSON key/value node, completed metadata record and pinned actual fixed core
RNG draw. This is admission work, not a callback meter inside existing Simplex.
The checked original census requires338695 +14466 +65 +1310 =354536 units.
Reject354535 before construction and admit354536 exactly; a successfully
constructed object reports its spent construction work. No CPU-cycle claim
follows from these units. Audit the runtime decoder against the same checked
source census and reject any drift before publishing a partial table. Before
parsing/constructing anything, compare the allowance against354536; constructor
seeds/octave shapes are closed, and original permutations/offsets plus final
Legacy state test the unchanged fixed draw chronology. Parsing reports the
observed exact byte/node/record totals before successful publication; original
draw census remains labelled original evidence rather than runtime instrumentation.

## Cache and MutableBlockPos boundaries

Base public `getTemperature` owns a per-biome/per-thread1024-entry insertion-
order cache. Its long key is only BlockPos.asLong (26/12/26 truncated axes),
without sea level. Hits do not move entries; a full cache removes the oldest
inserted entry. Changing sea level at one position or changing positions to
packed aliases can therefore reuse a numerically obsolete temperature.
Moonrise replaces that entire public cache path with the private computation.
Do not label stateless results as retained Vanilla public-method parity.

An actual original MutableBlockPos reuse probe is required in the oracle:
reuse one Java object while changing X/Z/Y and sea level; compare private Base
and public Paper results at every invocation. A separate retained Vanilla
sequence captures repeated position/different sea level, Y+4096 and X/Z+2^26
aliases, and1025 distinct packed positions followed by eviction replay. Fresh
Vanilla objects/public cold-cache calls remain separately labelled. No injected
cache answer or guessed Java algorithm is acceptable expected output.

Rust receives immutable Copy BlockPos and retains no reference to the caller's
position. SurfaceRules.Context.updateY captures primitive XYZ inside a new
memoized biome supplier while using one scratch MutableBlockPos on invocation.
Its LazyY temperature condition invalidates on updateY and updateXZ. These
lazy context caches belong to the later surface consumer, not this producer.

## Genuine original oracle and acceptance

Before production, freeze this design, full Base/Applied sources, source/feature
patches, original mappings and the actual constructor/resource census. A newly
compiled JDK-only reflection helper invokes actual official Vanilla/Paper
methods. No rewritten Java PerlinSimplex, modifier, temperature or predicate
algorithm supplies expected values.

Required accepted outputs:

1. Actual builtin Registry getter climate values for all65 biomes in both
   artifacts: raw f32 base bits, precipitation and actual modifier identity.
2. Actual fixed PerlinSimplex constructors, factors, all five permutations/
   offsets and raw f64 samples with offsets=false, plus genuine getCount draws.
3. Genuine private getHeightAdjustedTemperature f32 bits in both originals for
   every builtin biome. Genuine public Paper temperature/rain/cold/precipitation/
   iceberg results provide the production predicate expectations.
4. Real reused MutableBlockPos calls and retained Vanilla cache diagnostics,
   distinctly labelled with each stateful sequence and fresh/cold baseline.

The exact machine-readable contract is
`.codex-tmp/gen05_7/design/cases.json`, emitted by its pinned
`build_case_contract.py`. It contains93584 ordered scalar rows per mode:
87360 all65 ×16XZ ×12Y ×7sea Cartesian rows,3120 f32-horizontal precision
rows,1024 Frozen-grid rows and2080 sea+17 overflow rows. Duplicate coordinates
across labelled groups stay as separate invocations. No case is chosen from
an observed original output; Frozen branch coverage is reported from genuine
original noise/modifier probes rather than assumed from these coordinates.

There are80 calls reusing one actual MutableBlockPos for five representative
biomes, and3156 separately labelled retained-Vanilla cache calls for three
biomes. Each cache call records actual packed key, private calculation,
retained public result, a cold original public baseline and actual cache order.
Total labelled temperature rows are96820 per mode, below100000. Original
metadata/permutation/factor/sample records are separate from this row count.
The contract specifies exact call order, reset boundaries and counts before
accepted export. Private Base vs public Paper differences remain preserved,
even if unexpected, and are never normalized.

The runner pins canonical source/cases/guards, all original inputs and full
JDK; derives/checks Vanilla names from full official mappings. Reject the five
Java environment variables, the entire helper package in original artifact
ZIPs, noncanonical helper path, nonfresh outputs and wrong actual original/
library/helper CodeSources. Hash all inputs before/after. Normal and Python-O
exports and explicit require-based refusal series must agree. Declared digest/
ZIP-name fault injection remains separate from actual mutated artifacts.

Only after Root reviews genuine exports and pins should World runtime changes
begin. Runtime acceptance compares exact metadata/f32/f64 values, original
threshold predicates, constructor permutations/draw chronology and all labels
in Debug and Release. Include malformed private parser cases, type separation
from preset IDs/WorldSeed, exact construction allowance, capacity/clone and
allocation-free query evidence. Finish whole World/strict checks and exact
World-only archive review. No RSS/TPS, gameplay or server-completion claim.

## Connected continuation through surface output

1. This complete temperature producer removes the missing numeric dependency.
   Surface callers pass the actual resolved BiomeResourceId; no resolver or
   density/biome session phase is coupled into the temperature table.
2. Add a bounded original stored-quart resolver: .6 zoom coordinates plus
   explicit consistent complete neighboring .5 arrays, local X/Z masking and
   original ChunkAccess Y clamp. Surface build receives the WorldGenRegion
   manager, whose source uses BIOMES chunks; the direct-source replacement is
   in applyCarvers. Missing chunks require either a separately verified actual
   fallback source or explicit typed refusal as FerrumC policy, never Plains.
3. Compile all seven builtin surface_rule payloads into checked bounded original
   sequence/condition/block/bandlands rules. Implement original positional RNG,
   surface/secondary noises, clay bands and all11 condition kinds. Preserve
   per-XZ/Y lazy chronology, preliminary four-corner interpolation, vertical
   anchors and temperature predicates; flat-bedrock configuration is explicit.
4. Execute the genuine SurfaceSystem column driver against owned complete
   material states and WORLD_SURFACE_WG heights. X/Z order, descending Y,
   air/fluid resets, any dry nonair stone depth, exact default-state equality,
   eroded-badlands prepass, frozen-ocean postpass, evolving heights and fluid
   postprocessing must match. Return one complete checked owned surface-state
   output; failed execution cannot publish a partial terrain array.
5. An actual original ProtoChunk/SurfaceSystem corpus must compare complete
   states, heightmaps, fluid markers and selected chronology after original
   createBiomes/material filling. Genuine condition/endpoint probes supplement
   those full arrays. Only after surface/bedrock acceptance proceed to carvers,
   features/structures and eventually an activated natural-world identity.

The later surface implementation still needs its own connected design and
actual-original oracle; this source review alone does not approve that runtime.
