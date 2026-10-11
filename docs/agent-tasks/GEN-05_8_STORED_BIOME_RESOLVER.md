# GEN-05.8 — complete borrowed stored-biome resolver

Preparation delivery, 11.10.2026. Contract and oracle review belong to Root;
this document grants no runtime authorization. Only this new document and
`.codex-tmp/gen05_8/**` were written. GEN-05.5 arrays, GEN-05.6 zoom,
GEN-05.7 production/fixtures and all historical evidence remain immutable.

## Accepted basis and connected producer

Root's readonly accepted Git archive is
`40488324d69563a588023b96f051a62b28accc6b`. Independently enumerated membership
and whole SHA-256 bytes match all 1263 entries of
`.codex-tmp/gen05_8-root-base/base.freeze.json`, whose SHA-256 is
`d5c4e8afe5b3d717ef1c2e0751c303870b1994137f9ff0a4001a2901637dffb3`.
All 234 accepted GEN-05.7 source entries (source freeze SHA-256
`53f2905165fb5f72958d991e28f3fa2573152d5790d9c611cb8ecd1490cdc2d4`) match both
the archive and the shared tree; the shared `ferrumc-world` crate equals the
archive byte-for-byte apart from four untracked `__pycache__/*.pyc` interpreter
caches (`.codex-tmp/gen05_8/resume-audit/round3/receipt.json`). Current main
`e22b33389a216239b64b9b6e9542195bf88bd1e6`, as reported by Root, carries this
World runtime and does not replace the baseline. Accepted dependency pins come
from that archive, never from shared Sim/Storage WIP.

Two earlier preparation rounds were interrupted by usage limits. Their
artefacts are preserved, not overwritten: round 1 in
`resume-audit/round1/` and `round1-brief.md`; round-2 inputs in
`resume-audit/final02-inputs/` and `final02-input-pins.json`; exploratory and
superseded exports as `scratch-*`. None of them is a prior acceptance or freeze.
The authoritative candidate is the round-3 freeze described at the end.

The next World runtime may add one resolver module and its tests/reference
evidence. It may not edit .5/.6/.7 producers, RNG, dependencies, Registry,
world-source identity, another crate or App activation.

This producer connects actual complete `ChunkBiomeSamples` to `BiomeZoom`. It
supplies resource identities to the later surface-rule compiler and complete
SurfaceSystem driver described in GEN-05.7's connected continuation. It
performs no biome/density generation, surface mutation, chunk installation or
persistence.

## Concrete full API candidate

All fields are private and every public item requires rustdoc.

- `StoredBiomeIdentity::new(WorldSeed, NoiseSettingsResourceId,
  BiomeSourceIdentity, BiomeQuartGeometry) -> Self`, with read-only `seed()`,
  `resource()`, `source()` and `geometry()`. These existing opaque checked
  types are the identity; there is no string, digest or raw-array authority
  constructor.
- `StoredBiomeBuildBudget::new(u32) -> Result<Self, StoredBiomeError>` admits
  `1..=1_048_576` units and exposes `units()` without mutation.
- `StoredBiomeQuartPos::new(i32, i32, i32) -> Self` admits all signed32 quart
  axes; `from_zoom(BiomeQuartPos) -> Self` preserves the selected quart bits.
  `x()/y()/z()` expose them. It is distinct from `BlockPos` and density's
  `ClimateQuartPos`, whose quart-to-block admission is a different contract.
- `StoredBiomeNeighborhood<'a>::new(identity, center: ChunkPos,
  chunks: &[&'a ChunkBiomeSamples], budget) -> Result<Self, StoredBiomeError>`
  accepts 1..=9 complete arrays inside the center's 3×3 window. It copies at
  most nine immutable references into `[Option<&'a ChunkBiomeSamples>; 9]` and
  retains no outer slice, collection, `Arc`, source session, callback or cache.
  Clone/Copy is a bounded value copy. `identity()`, `center()`,
  `chunk_count()`, `borrowed_value_count()`, `construction_work()` and
  `native_retained_bytes()` expose admitted metadata. No mutable reference or
  replacement operation exists.
- `noise_biome_at_quart(StoredBiomeQuartPos) -> Result<BiomeResourceId,
  StoredBiomeError>` selects the actual signed chunk and stored slot.
  `noise_biome_at_block(BlockPos)` performs the original unzoomed `>> 2`
  conversion on every axis, then that lookup. `biome_at_block(BlockPos,
  BiomeZoomSemantics)` composes the existing .6 `BiomeZoom::select_quart` with
  the same stored lookup. The neighborhood derives exactly one `BiomeZoomSeed`
  during construction via `BiomeZoomSeed::from_world_seed(identity.seed())`
  and retains it; numerical semantics stay explicit per block query. No public
  operation accepts an independently supplied or already-obfuscated zoom seed,
  so the zoom seed cannot mismatch the arrays and is never hashed twice.
- `SurfaceBiomeResolver<'a>::new(identity, center, chunks, semantics, budget)`
  requires all nine distinct chunks, including the center, and owns a
  neighborhood plus explicit semantics. Its `biome_at_block(BlockPos)`
  additionally requires X/Z inside the center's complete 16×16 block column;
  every signed32 Y is supported. `neighborhood() -> &StoredBiomeNeighborhood<'a>`
  gives read-only access (including `noise_biome_at_quart`). `identity()`,
  `center()`, `semantics()` and `construction_work()` expose state.

### Why a complete 3×3 halo (source review plus differential evidence)

Design argument: for any block in a center column, .6 subtracts 2 and selects
X/Z from the enclosing shifted quart or its +1 corner. Block X in
`[16cx, 16cx+15]` therefore selects quart X in `[4cx-1, 4cx+4]`, i.e. chunk
`cx-1..=cx+1`, for every Y and seed; Z likewise. Complete surface-column
support follows without evaluating surface rules or predicting the winner.

Source review (not execution): the actual 1.21.8 `ChunkPyramid` generation
step for `SURFACE` requires `STRUCTURE_STARTS` radius 8 and `BIOMES` radius 1,
identically in Base and Paper (Paper's Moonrise hunk changes only `LIGHT`), and
`ChunkStatusTasks.generateSurface` constructs the `WorldGenRegion` whose
manager `NoiseBasedChunkGenerator.buildSurface` passes to `SurfaceSystem`.
Whole Base/Applied copies are reconstructed exactly from both patch layers in
`resume-audit/supplement-layers/receipt.json`.

Differential evidence: every one of the 20 genuine `createBiomes` groups
answers all 3840 center-column `LevelReader.getBiome` queries and all 360
direct quart queries from its own nine real chunks with outcome `value`;
the runner refuses any other outcome for these groups.

## Identity, completeness and independent histories

Every array must agree exactly with the expected WorldSeed, checked
`NoiseSettingsResourceId`, explicit `BiomeSourceIdentity` and complete accessor
geometry. Each chunk key must be unique and inside the admitted 3×3 window.
The center and each supplied chunk must satisfy the existing .3 `admit_chunk`
domain used by every .5 producer: origin `16·c` and interpolation edge
`16·c+16` inside `[-30_000_000, +30_000_000]`, computed in i64 (not merely the
last stored block `origin+15`). At this producer edge a zoom-selected neighbor
may be missing: sparse lookup returns `MissingChunk`, complete surface
construction refuses the incomplete halo. No all-i32 .6 result extends .5
generation admission or authorizes a fresh source. A biome source is never
inferred from settings or from an arbitrary label or hash.

Only .5's sealed successful `BiomeSourceSession::sample_chunk_biomes` produces
the public array. Its fields are private and there is no Clone, raw-array
constructor or importer. The expected identity copies existing checked types;
an identity alone is never evidence of completeness. The public resolver only
accepts references to these sealed objects, which guarantee complete checked
values, original creation order and atomic publication. Admission nevertheless
checks defensively:

- `values().len() == geometry.sample_count()`, 64..=2048 values, section
  alignment and checked top (the `BiomeQuartGeometry` invariants);
- Fixed and End: both history endpoints `None`; Fixed additionally requires
  every value to equal its checked fixed ID;
- MultiNoise: a final selection is required; the initial selection may be
  `None` (cold) or a valid pre-array selection. Each present selection's
  `parameter_ordinal()` must index the source preset's
  `BiomeParametersResourceId::parameters()` (7593 Overworld, 5 Nether) and that
  entry's `biome()` must equal the selection's `biome()`;
- the final selection's biome must equal the value in the last execution slot
  (top quart Y, local X 3, local Z 3), which is also the last storage slot.

These are O(1) per array apart from the value scan; no tree construction,
search or history replay occurs. Closed checked IDs and producer-owned leaf
ordinals are trusted provenance, not reconstructed from names or source
re-evaluation. Geometry admission in .5 additionally requires containment of
the full noise interval, so arbitrary checked geometry cannot bypass that
producer. The seven axis-control geometries below exercise the lookup kernel;
the extreme i32 and narrow accessors are not claimed to be genuine .5 outputs.

Histories need not be equal across chunks, and one chunk's final selection
need not equal another's initial selection. Separate cold sessions, shared
retained forward/reverse sessions and separately scheduled valid producers are
all legitimate (the oracle retains cold, forward and reverse groups). The
resolver freezes each complete array as supplied; it does not promise
schedule-independent regeneration and never resets or replays history. Mixing
different settings, source, seed or geometry is refused even where the
resulting values happen to match. Different original numerical outputs remain
distinct oracle rows. .5 produces Paper-context arrays; Vanilla originals
additionally diagnose the common stored getter and numerical-mode differences.

## Exact stored lookup and refusal order

For quart `(qX, qY, qZ)` select `ChunkPos(qX >> 2, qZ >> 2)` using arithmetic
shifts. Compute window deltas `dx, dz` in i64 and one fixed slot
`(dx+1)*3 + (dz+1)`. Refuse `OutsideNeighborhood` first, then
`CoordinateDomain` if the selected chunk fails .3 `admit_chunk`, then
`MissingChunk` if the slot is empty. Quart axes retain the full i32 input
domain; this selected-neighbor admission is separate from .6's full-i32
mathematics. `MissingChunk` records the exact typed key; neither lookup nor
construction substitutes Plains, a nearest chunk, a fresh source, fallback
climate or a partially filled array.

Let `minQuartY = minY >> 2` and `maxQuartY = minQuartY + (height >> 2) - 1`.
Clamp only Y to this closed interval, including both original height ends.
Local X/Z are `qX & 3` and `qZ & 3`. The section is
`(clampedQuartY >> 2) - (minY >> 4)` and local Y is `clampedQuartY & 3`. The
actual .5 storage index is
`((clampedQuartY - minQuartY) * 4 + localX) * 4 + localZ` (quart Y/X/Z order),
exactly `ChunkBiomeSamples::values()`'s documented order. Original Java palette
storage is Y/Z/X (`PalettedContainer.Strategy.getIndex`); that order is
separate oracle evidence and the resolver never reads Java palette indices or
wire IDs. Coordinates are never multiplied from an unbounded quart into a block
i32. Vanilla's getter clamps then calls `getSectionIndex(QuartPos.toBlock(y))`;
Paper's getter clamps the section index directly. On admitted section-aligned
complete geometries both produced identical values for every oracle row.

Block methods first refuse X, then Z, outside `[-30_000_000, +30_000_000]`.
This is a FerrumC policy refusal, not original behaviour: the oracle's
`fulli32` rows retain what the originals actually delegate at ±30_000_001 and
the i32 extremes. Surface then refuses X/Z outside the center column before .6
selection. All block Y bits are admitted; selected Y is clamped only by the
stored lookup. .6 retains its original wrapping subtraction and strict corner
chronology, so a block Y near `i32::MIN` may legitimately select the top quart
(retained oracle rows show this). No world-border or height normalization
occurs before selection. Already-obfuscated seeds remain an oracle control and
an existing .6 API, never an independent resolver identity.

Construction refusal order: the budget is already a validated type; then
count 1..=9; center domain and full window arithmetic; retained/scratch
ceilings; then each supplied array in caller order: window/key domain,
duplicate, seed, settings, source, geometry, complete length and history
shape. The whole work requirement is computed before any value scan or seed
hash; an insufficient allowance is refused there. Then every value is scanned
(Fixed equality, final MultiNoise slot), private references are installed and
the zoom seed is derived once. Surface finally checks all nine required keys in
slot order: their .3 domain first, then presence. An edge center can be
admitted sparsely while complete surface construction refuses an
out-of-domain halo: full surface centers lie in `[-1_874_999, 1_874_998]` on
both axes, sparse centers in the complete .5 range `[-1_875_000, 1_874_999]`.
Only a complete candidate is published; no caller array, history or work
counter is mutated on any failure.

`StoredBiomeErrorKind` is concrete: `InvalidBudget`, `InvalidChunkCount`,
`CoordinateDomain`, `OutsideNeighborhood`, `DuplicateChunk`,
`InconsistentSeed`, `InconsistentSettings`, `InconsistentSource`,
`InconsistentGeometry`, `IncompleteArray`, `InvalidHistory`,
`InvalidFixedValue`, `MissingChunk`, `OutsideSurfaceColumn`,
`WorkBudgetExceeded`, `NativeLimit`, `InvalidStoredIndex`. The error carries
an optional typed chunk/quart/block, optional input ordinal, optional
`StoredBiomeLimit` and requested/maximum counts, read via `kind()`, `chunk()`,
`quart()`, `block()`, `input_index()`, `limit()`, `requested()` and
`maximum()`; it allocates no message and invents no fallible-allocation error.
Limits are `Chunks`, `ValuesPerChunk`, `BorrowedValues`, `RetainedBytes`,
`ScratchBytes` and `ConstructionWork`. `InvalidStoredIndex` is defensive
against private corrupted test controls, not a claimed valid original failure.

## Bounded resource and runtime acceptance contract

Caps: 9 borrowed arrays; 2048 values per array; 18432 borrowed values total;
512 bytes retained per neighborhood or surface resolver; 4096 bytes of
simultaneously live construction scratch; zero retained, query or build heap
allocations. Borrowed array object/value capacities are counted separately
using actual `size_of` and lengths and are never charged as newly retained
resolver heap or described as RSS. The .5 objects keep owning their
allocations for lifetime `'a`.

The design-only field-layout probe (`.codex-tmp/gen05_8/layout/`) compiles the
actual accepted dependency types from a private copy of the entire 1263-file
archive plus one example file. On this x86_64 / Rust 1.99.0 toolchain Debug and
Release report identical sizes: identity 24, neighborhood 136, surface 144 and
error 80 bytes; each borrowed array object 56 bytes, resource ID 1 byte,
reference 8 bytes. Maximum borrowed object/value capacity is
504 + 18432 = 18936 bytes, separate from the retained 136/144 bytes. No
source, tree or noise session is retained. Conservative simultaneously live
typed build objects are 560 bytes plus a reserved 1024-byte SHA/call-temporary
allowance, below 4096. This is a layout prototype and design census, not an
implemented resolver stack or allocation measurement.

Construction admission work is exactly `2 + 10·N + total_values`, plus 9 for
surface coverage: two units for candidate start/publication; ten per entry for
window/key, duplicate, seed, settings, source, geometry, shape, history,
endpoint and installation admissions; one scan unit per stored value. This is
a declared admission census, not CPU cycles. The minimum is 76 (one
64-value array), the maximum 18524 (neighborhood) or 18533 (surface). An
allowance below the requirement fails before seed hashing or value traversal;
exact and one-above allowances succeed and report the exact used work. Hashing
is the existing bounded one-block .6 producer, not a query RNG draw or source
work charge.

Quart lookup performs two shifts, constant bounded window/index arithmetic,
one slot read and one value read. Unzoomed block lookup adds three shifts.
Zoomed block lookup executes the existing exact eight corners / 64 LCG steps
once, then one stored lookup. No query loop depends on supplied count or
height; queries perform no allocation, IO, callback, density history, cache
mutation, sampler budget or poison state. Later surface sessions meter their
own call sequences; the resolver is constant-bounded stateless work and has no
cumulative query meter.

Runtime must compare every approved genuine query, resource and
delegated-coordinate row in Debug and Release, including both height clamps and
all array indices. Integrate real .5 arrays and the .6 selector in genuine full
nine-chunk neighborhoods; retain the original separate cold and shared
forward/reverse histories and test reordered borrowing. Synthetic axis data may
reach only the common private checked lookup kernel's `cfg(test)` view. Do not
fabricate `ChunkBiomeSamples` through sibling-field access, unsafe code, a
public raw constructor or a production .5 edit. The production query obtains
that kernel's geometry/value slice exclusively from its already admitted sealed
borrowed chunk. Axis tests check all original getter rows there; every real .5
public build/composed-query test uses actual .5 producers. Compare each actual
.6 selection with every genuine delegated quart and use the same
selection/lookup operation in the public composition.

Meaningful private malformed/policy controls cover 0/1/8/9/10 entries, each
missing halo slot, duplicate first/last/multiple defects with deterministic
refusal order, each inconsistent identity component, wrong/partial/extra array
length, invalid history (including ordinal/biome mismatch) and Fixed content,
1/32/33 sections, checked min/top and signed chunk extremes, block-domain and
neighborhood boundaries, all Y extrema, caps at N-1/N/N+1, exact 75/76/77
minimum-array and 18523/18524/18525 and 18532/18533/18534 maximum-work
boundaries, invalid allowance 0/1/1048576/1048577, native-cap faults and
mutation-free failed publication. Borrow-checker and type-separation
`compile_fail` examples prove no detached borrowed resolver, wire ID,
WorldSeed/zoom-seed or density-quart substitution.

Measure actual complete layouts and simultaneous scratch in the final runtime.
Use separate cold Debug/Release allocator processes covering the first
admitted and refused builds, clones and whole-corpus queries, with a positive
4096-byte allocation control. No unsafe Rust, dependency or production global
is added; external test instrumentation follows Root's accepted .7 cold
measurement model. Finish whole-World Debug/Release, strict all-target Clippy,
workspace fmt and warnings-as-errors rustdoc on the exact Root archive plus the
whole World overlay; verify baseline membership and all outside-file bytes
before and after. Original fixtures and final source stay unchanged during all
final checks.

## Genuine original preparation protocol

### Source review (read, pinned, reconstructed; not executed)

Complete Base and Applied sources plus both patch layers for BiomeManager,
LevelReader, WorldGenRegion, ChunkAccess, ProtoChunk, LevelChunkSection,
PalettedContainer, QuartPos, SectionPos, LevelHeightAccessor,
NoiseBasedChunkGenerator and SurfaceSystem, supplemental callees (BiomeSource,
MultiNoise/Fixed/End sources, Climate, NoiseChunk, RandomState, SurfaceRules,
BlockPos, ChunkPos, Util, TracingExecutor, WorldUtil) and, in round 3,
ChunkPyramid and ChunkStatusTasks. Whole copies, source maps and exact
reconstructions live in `.codex-tmp/gen05_8/sources/` and
`resume-audit/{layers,callee-layers,supplement-layers}/`. Selected class
diffs accompany whole feature patches and are not whole-method authority.

- `WorldGenRegion` constructs its manager from itself plus
  `BiomeManager.obfuscateSeed(level.getSeed())`, the same world seed whose
  `RandomState` produced the arrays.
- `LevelReader.getBiome` calls that manager; `getNoiseBiome` requests
  `getChunk(qx>>2, qz>>2, BIOMES, false)` and calls the stored `ChunkAccess`
  getter, or the abstract `getUncachedNoiseBiome` hook if null.
- `WorldGenRegion.getChunk` throws a `ReportedException` for any chunk outside
  its step's dependency radius or not yet at the required status, even with
  `requireChunk=false`. This preparation does not execute a
  WorldGenRegion/ServerLevel; the genuine LevelReader default-method proxy is
  labelled exactly that helper scope, including controlled hook observations.
- Surface build passes the region manager unchanged into `SurfaceSystem`; its
  direct and lazy rule biome queries use `BiomeManager.getBiome` on blocks of
  the center column. `applyCarvers` alone uses `withDifferentSource` with a
  direct `BiomeSource`/`random.sampler()` replacement. No carver call, source
  fallback, SurfaceSystem execution or server invocation is claimed.
- Paper replaces the whole `ChunkAccess.getNoiseBiome` with direct section
  calculation and drops Vanilla's `ReportedException` wrapper; Paper's
  `BiomeManager.getFiddle` uses the integer-midpoint form (.6's semantics).
  Vanilla runs `createBiomes` on the background executor, Paper on the calling
  thread; the harness reads the thread-local RTree history on the actual
  executing thread in each mode.

### Differential evidence (actual original execution)

The freshly compiled JDK-only helper `oracle/StoredBiomeOracle.java` invokes
original constructors, actual sections/palettes/setters/getters, genuine
`BiomeManager` and actual `LevelReader` default methods through pinned official
mappings (Vanilla obfuscated JAR) and Paper's Mojang-mapped JAR. Axis controls
use the 65 actual builtin resource holders: 64 distinct cells per section,
rotated per section and chunk across the roster. Population arithmetic only
assigns explicit inputs; every expected value is an original getter output and
each value row carries the actual chunk, section and palette getter results and
the original `Strategy.getIndex`. Real neighboring `createBiomes` arrays come
from actual WorldPreset/DimensionType/RandomState/ProtoChunk/StructureManager/
Blender paths. The source delegates manager-selected coordinates exactly once;
rows record the delegated quart, requested chunk, resource identity and
source/chunk/hook/manager call counts plus the actual zoom-seed bits. No helper
implements getter, clamp, index or zoom arithmetic as expected output.

Declared corpus (`design/cases.json`), identical in both modes:

| Item | Count |
| --- | --- |
| Axis geometries (min/height): -64/384, 0/256, -32/64, 32/16, -2048/512, 2147483632/16, -2147483648/16 | 7 × 9 chunks |
| Genuine `createBiomes` groups (normal, large_biomes, amplified, nether, end at chunk 65, single_biome_surface; seeds 0, -1, i64 min/max; cold/forward/reverse; accessors -80/416 and -64/512) | 20 × 9 = 180 chunks |
| Original arrays / stored getter value rows | 243 / 300096 |
| Distinct controlled axis cells (each also a direct quart query) | 45504 |
| Query rows (`G` LevelReader.getBiome, `B` getNoiseBiomeAtPosition, `Q` getNoiseBiomeAtQuart) | 220960 |
| Outcomes per mode | 215392 `value`, 5568 `controlled_null_hook` |
| Diagnostic getter probes (before BIOMES, null section, unaligned/zero-height accessors) | 32 |
| Seeds | world 0, -1, i64 min/max; obfuscated 0 and 2^28 as axis controls only |

Results: all 37 cold Paper arrays that overlap the accepted .5 fixture
(`biome-arrays-original-1.21.8/arrays-compact.jsonl`) match it in whole values,
quart membership and final ordinal. Vanilla and Paper agree on every value,
query and history row. Exactly 21 diagnostic rows differ, all outside
`BiomeQuartGeometry` admission: 10 value rows on unaligned accessors
(-63/17, 1/17) where Paper's section shortcut differs from Vanilla's clamp,
10 zero-height accessor exceptions (Vanilla's `ReportedException` wrapper and,
for negative Y, Paper's different failing array index) and the null-section
fault (wrapper and mapped names only). Shared histories continue exactly
from the previous chunk's final ordinal; cold groups start at `none`;
Fixed/End groups carry no leaf history. Exact counts are re-derived by the
runner from original outputs; see `final-normal/manifest.json`.

Vanilla/Paper zoom numerical modes produced no differing query in this corpus;
.6's accepted corpus remains the authority for their divergence, and .8
runtime evaluates each row under its own explicit mode.

### Runners, pins and guards

- `oracle/prepare.py` declared the inputs and case counts before execution.
  `oracle/input-pins.json` pins the whole 1263-file archive, accepted .5/.6/.7
  original classes/JARs/libraries/mappings, the complete JDK membership, all
  sources and design inputs, and the canonical helper/runners. Round-2 and
  round-3 refinements are declared, backed up and receipted
  (`delivery/pin-refresh-receipt.json`, `delivery/pin-refresh-round3-receipt.json`):
  the diagnostic-log manifest split, the guard's `-O` propagation and the
  supplemental ChunkPyramid/ChunkStatusTasks pins. Nothing else changed.
- `oracle/run_oracle.py OUT` refuses the five Java environment variables,
  helper-package shadows, noncanonical helper source, any pinned byte or
  membership drift (before and after export) and nonfresh output; it checks
  actual original, library and helper CodeSources inside the JVM.
  Timestamped Minecraft bootstrap/stdio logs are listed in
  `diagnostics-manifest.json` and never compared as differential evidence.
- `oracle/project_originals.py` mechanically projects original outcomes to
  JSONL (`projection/{arrays,queries,histories,exceptions}.jsonl`) without any
  numerical expectation formula. Policy refusals and malformed-input controls
  are not oracle rows and stay separate in the runtime tests.
- `oracle/check_guards.py OUT REF` exercises 15 guards (5 environment via the
  real entrypoint, canonical source, digest, helper shadow, fresh output, write
  scope, JDK and archive membership, and three real-JVM CodeSource mismatches
  refused before any sample), all with explicit `require`, in the same Python
  mode as the caller. The two `guards.json` files differ only in the recorded
  `entrypoint_python_optimize` field (0 versus 1); the guard results are equal.

Fresh round-3 runs: `python3 run_oracle.py final-normal`,
`python3 -O run_oracle.py final-optimized`, and the matching
`check_guards.py` runs into `guards-normal`/`guards-optimized`.
`delivery/freeze.py` re-verifies every pin, compares all differential outputs
and reports byte-for-byte and the guard results, writes
`delivery/final-compare.json`, then the whole-file
`delivery/freeze-manifest.json`. Its SHA-256 is reported to Root separately.

## Open points and limitations

- Runtime fixtures: the full projections are about 36 MB of query JSONL per
  mode. The runtime work order must define a compact mechanical projection
  (as .5's `arrays-compact.jsonl`) plus its validator; this preparation does
  not commit fixtures into the World crate.
- The layout figures are a design census on one toolchain; retained size,
  scratch and allocation-free queries remain to be measured in the runtime.
- No WorldGenRegion, ChunkMap, SurfaceSystem, carver or server execution; the
  halo and surface-caller statements above are source review.
- Block X/Z domain refusal and the full 3×3 surface halo are FerrumC policy;
  a genuine fallback producer for missing chunks remains future work.
- Root must independently reproduce and review this contract and oracle
  before CLEAN_CONTRACT/CLEAN_ORACLE and a separate runtime work order. No
  runtime gate, Survival, Java plugins, natural-world activation, TPS, RSS or
  latency claim follows from this document.
