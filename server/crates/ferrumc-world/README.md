# ferrumc-world

Pure world model. No threads, no DB, no packets.

The seed-free `density` module now compiles every original Minecraft 1.21.8
builtin density definition and all seven builtin settings routers into checked,
immutable private graphs. `DensityTemplate::compile_definition` and
`RouterTemplate::compile_builtin` expose geometry, flags, noise references and
graph metadata. They validate all 35 definitions, including unused graphs.
`RouterTemplate::bind_seed` now constructs an immutable `SeededRouter` for
explicit seeds. It samples the fifteen original router fields, all 35 named
definitions and six climate axes in the original empty-Blender point context.
The executable server defaults to the flat generator and offers an explicit
natural terrain preview that composes these producers through carvers, as
described below.
The separately bounded `SeededRouter::fresh_paper_noise_chunk` now executes
original Paper interpolation, cache wrappers and one complete material-density
loop. Its `sample_chunk_materials` alternative follows aquifer, vein and
settings-default selection in the same driver and returns complete states and
fluid postprocessing bits before surfaces, carvers or features.

Admission retains both direct/reference `HolderHolder` nodes and distinct raw
spline-coordinate holders. Bounds are 2 MiB distinct original source bytes,
4096 vertices, 8192 edges and combined depth 96. The original spline-local
limits remain separate. Decimal tokens round directly to the original f64/f32
codec widths, including signed zero. INT fields truncate/wrap with the pinned
Gson rules before range validation. Required spline derivatives, resource roles
and original geometry guards are checked. Extra field/finite/ascending-point
restrictions are explicit `FerrumC` policy. Original inline noise parameters
remain outside the closed builtin resource API. The material alternative
admits deferred default block/fluid payloads through the existing pinned state
registry. Surface rules and spawn targets remain unexecuted payloads.

Structural fixtures decode the real original JAR's codecs and compare all
35 definitions, seven routers and 32 node forms; this establishes no sampled
terrain, Survival or Paper runtime parity. See the
[density oracle](tests/reference/density_compiler.md) and
[GEN-05](../../../docs/agent-tasks/GEN-05_DENSITY_COMPILER_ROUTER.md) for the
exact projection scope, corrected holder inventory and next consumers.

Seed binding distinguishes decoded codec objects from actual native registered
`NoiseRouterData` targets. Sparse native variants preserve factory provenance
and original first/second visitor diagnostics inside the existing graph caps.
Bounds preserve original nonfinite, reversed and signed-zero diagnostics;
they are metadata rather than sample clamps. Sampling rejects a nonfinite
completed requested node with a classified error and no replacement density.

Every point or six-axis climate sample shares an explicit work budget of
1..=1,048,576 across density, spline and noise operations. Batches accept
1..=4096 requests with one aggregate budget up to 67,108,864 units and publish
results only after all requests succeed. Horizontal block coordinates include
±30,000,000; vertical coordinates support all i32 values. Checked quart
coordinates convert to the original block origin without Java shift wrap;
negative unaligned blocks floor to their enclosing quart. Exact block climate
sampling is also available. These samples contain no biome or spawn selection.

The separate [seeded router oracle](tests/reference/density_router.md) calls
actual original `RandomState`, both visitors, scalar/bounds and climate methods.
Its corpus covers all settings and signed seed extremes, nested spline
properties, native registered targets, direct large per-octave wrapped noise,
canonical-name controls, aquifer/ore positional draws and selected-child traces.
Original Java NaN/throw behavior and `FerrumC`'s checked refusal policy are
compared separately. Vanilla's overflow-prone distant End density is retained
as this oracle's numerical reference; Paper's widened End path needs separate
acceptance before an active terrain generator claims Paper parity. No RAM/TPS
improvement or Survival/plugin completion follows from scalar parity.

The [fresh Paper noise-chunk corpus](tests/reference/noise_chunk.md) compares
126 complete density arrays and original constructor/cache/provider/Climate
traces. Sessions own all mutable caches, share one explicit construction/query
work budget, reject illegal phases before mutation, poison after query failures,
and rebuild a complete candidate before reset. Public point/noise streams keep
their original Vanilla policy; the chunk consumer privately follows Paper
Build 60's widened End radius and float-rounded random offsets. The material
alternative returns owned states and fluid bits; it does not activate a
natural-world identity or install them into a live chunk. The
[material corpus](tests/reference/material_chunk.md) compares all126 complete
actual-Paper arrays (7,077,888 states and flags),499 independent finite endpoint
cases and42 enabled-aquifer/cache/tie cases, including original request/RNG
chronology and cache snapshots. Fifteen nonfinite original-Java probes instead
verify Rust's additional typed refusal policy. All541 finite endpoint cases
have separate exact shared-budget boundaries.

The separate `biome_selection` consumer builds the exact original builtin
`RTree` and owns its ordered selection history explicitly. `BiomeSourceSession`
supports checked multinoise, fixed resources and the original five-biome End
source. `sample_chunk_biomes` fills a complete explicit `BiomeQuartGeometry`
before material execution, including the full 256-block Nether/End accessor
above their 128-block noise settings. It never infers a biome source from
settings or publishes a partial array. A rejected executed operation retains
prior multinoise history and poisons both sessions until explicit reset.
The [biome-selection oracle](tests/reference/biome_selection.md) compares both
complete original trees, all3287 retained-history/tie queries,156 actual
`createBiomes` arrays and27 original End radius/f64-threshold probes. Output
identities are checked local resources, with no wire/persisted-ID assignment.
Source/session scheduling, custom parameter/source producers, surface rules,
carvers, features, structures and active natural worlds remain separate work.

The immutable `biome_zoom` producer now implements original seed obfuscation
and block-to-quart biome zoom for every signed32 block axis. `BiomeZoomSeed`
is distinct from `WorldSeed`, and its output-only `BiomeQuartPos` is distinct
from checked density coordinates. Explicit Vanilla/Paper contracts preserve
wrapping coordinates, LCG steps, distance arithmetic and first-minimum ties.
Selection evaluates exactly eight corners without allocation or source calls.
The [biome-zoom oracle](tests/reference/biome_zoom.md) compares actual original
hashes, all1024 fiddle buckets,49 LCG controls,6815 selections and54520 separate
distance probes in each mode. Four genuine equal-minimum cases select the
first corner; both original modes have identical value bytes in this corpus.
This computes coordinates only; eventual source resolution and original
surface rules remain separate consumers.

This crate owns the world's block-storage value types and a flat-world
generator:

- `BlockStateId` — a newtype over the protocol block-state id (default: air).
- `PackedArray` — a bit-packed array of fixed-width entries (Minecraft
  chunk-storage layout, non-spanning).
- `PalettedContainer` — block-state storage with an automatically promoted
  palette (single value → indirect palette → direct ids).
- `ChunkSection` — a 16x16x16 block container indexed by `LocalBlockPos`.
- `Chunk` — a full-height stack of `SECTION_COUNT` (24) sections spanning the
  overworld (`MIN_Y = -64`, height 384). Addressed by absolute `BlockPos`, with
  per-section dirty tracking (`DirtySections`), per-column `Heightmap`s, and
  documented placeholders for lighting (`ChunkLight`) and block entities
  (`BlockEntity`).
- `FlatWorldGenerator` — deterministically fills a `Chunk` with a flat overworld
  profile: bedrock floor, stone fill, dirt, grass surface, then air.
- `WorldSeed`, `GenerationIdentity` and `ChunkGenerator` — explicit numeric seed
  and versioned identity for pure, reproducible chunk generation. The current
  identities are Minecraft 1.21.8 / Overworld, with flat revision 1 and natural
  terrain preview revision 1.

## Generator contract

`FlatWorldGenerator::new()` and `default()` select numeric seed 0 and preserve
the existing fixed layers. `with_seed(WorldSeed::new(value))` retains another
signed 64-bit seed in its identity. Flat blocks remain seed-independent; these
seeds still identify different baselines. Both the existing `generate` method
and object-safe `ChunkGenerator::generate_chunk` use the same production path.

`GenerationIdentity::encode` returns a fixed 27-byte `FCGI` schema-1 envelope.
Its big-endian fields are magic, `u16` envelope schema, three `u16` Minecraft
release components, `u16` algorithm, `u32` generator revision, `i64` seed and
`u8` dimension. Algorithm 1 names `FerrumC` flat, algorithm 2 names its natural
terrain preview; dimension 0 names Overworld.
Envelope schema and generator revision are independent. `decode` requires the
exact length, allocates nothing, and classifies unsupported metadata without
falling back. `ensure_compatible` requires exact supported identity equality.
This envelope is `FerrumC` metadata, not Vanilla NBT or protocol data.

The production App/Sim/Storage path now persists and verifies this identity
before regeneration and overlay application. Existing legacy worlds require
the explicit offline generation migration; they receive no inferred seed.
See [WG-004](../../../docs/agent-tasks/WG-004_APP_GENERATION.md) for the consumer
contract. `RuntimeWorldGenerator` now exposes both the existing flat profile
and the natural Overworld terrain preview described below. Nether/End and
migrations to future generator algorithms/revisions remain separate work. See [WG-001 notes](../../../docs/agent-tasks/WG-001_GENERATION_CONTRACT.md)
for pinned reference behavior and consumer dependencies.

## Original random and noise primitives

`random` implements the explicitly selected original Legacy48 and
Xoroshiro128++ streams, numeric/128-bit seeds, fork/reseed and immutable
coordinate/name factories. Names follow Java UTF16 hashing for legacy and MD5
of UTF-8 for xoroshiro. Gaussian sampling and Java strings with isolated
surrogates remain outside this API. Invalid bounds preserve the stream;
explicit consume calls are capped at 1,048,576 steps.

`noise` provides `ImprovedNoise` (including original vertical quantization and
analytic derivatives) and `PerlinNoise` with explicit first octave/amplitudes,
modern named-octave construction or original legacy gap consumption. Sampling
allocates nothing and does not mutate the sampler. Construction accepts 1..64
amplitudes with exponents -64..32, finite amplitudes and finite calculated bounds;
legacy additionally rejects positive octaves. Coordinates and y parameters must
be finite with magnitude at most 2,147,483,136. These explicit `FerrumC` limits
reject unsupported configurations; they do not extend Vanilla's contracts.

Checked fixtures invoke the real original Minecraft 1.21.8 JAR and compare
16,556 records, full permutations and raw floating-point bits without tolerance.
Reproduction, exact pins and numerical scope are in
[GEN-04.1](../../../docs/agent-tasks/GEN-04_1_RANDOM_NOISE.md).
These modules activate no natural terrain and add no executable generator
identity.

`SimplexNoise` now supplies the original 2D/3D sampler, including equality
branches and Java integer overflow. Its offsets are diagnostics; the original
samplers do not add them to input coordinates. `NormalNoise` supplies original
modern or explicit legacy two-stack sampling and nonzero-amplitude-span
normalization. Its caller coordinate cap is
`MAX_NOISE_COORDINATE / 1.0181268882175227`, with the second scaled coordinates
also checked. These are explicit `FerrumC` limits, not Vanilla defaults.

`BlendedNoise` supplies original 16/16/8 legacy stacks, interpolation clamps,
bound diagnostics and `with_new_random` reconstruction. Its checked
`BlendedNoiseParameters` supports the original codec ranges: four factors in
`0.001..=1000`, smear multiplier in `1..=8`. The original Java testing constructor
does not enforce those ranges; this Rust API deliberately rejects that wider
domain. Sampling accepts the full typed `BlockPos` i32 domain and preserves
internal wrap/vertical semantics without applying public Perlin coordinate caps
to intermediate consumer values. No implicit unseeded/default constructor is
provided. All samplers allocate nothing and consume no random output while
sampling. Constructor errors preserve the supplied parent random state.

A separate original-JAR oracle covers 16,974 composite-noise records with exact
float bits, complete active permutations, signed seed extrema, parent RNG
consumption, normal scaling boundaries, Blended codec endpoints and clamped
interpolation boundaries. See
[GEN-04.2](../../../docs/agent-tasks/GEN-04_2_NORMAL_SIMPLEX_BLENDED.md).
`WorldgenRandom` feature/decoration seeding, Overworld/Nether/End chunk consumers
and complete world seed parity remain separate work. Seeded plain-point density
and End scalar sampling are supplied by the separate checked router consumer. Primitive goldens do not establish complete terrain or
cross-platform parity.

## Original cubic splines

`spline::CubicSpline` owns an immutable constant/multipoint DAG. Construction
checks every borrowed specification, including unused nodes, before allocating
the graph: at most 1024 nodes, 64 declared coordinates, 64 points per node,
16,384 total points and depth 32 (constant leaf = 1). Locations must be strictly
ascending; float inputs are finite; IDs are checked graph-local slice indices.
They are not persistent handles and cannot identify their construction of origin.
Clones share the immutable graph through `Arc`; no input slice is retained.

`sample_with` requires 1..=1,048,576 work units. Node entries, coordinate callbacks
and binary-search comparisons each charge one unit before execution. Sampling
performs no internal heap allocation, mutation, I/O or random draw. A caller's
callback owns its own effects and cost. Coordinates and shared children are
requested repeatedly, without caching. Original endpoint extensions evaluate
one child; interior intervals evaluate left then right, including exact interior
locations. All interpolation preserves the original f32 operation order without
deliberate FMA.

Reported coordinate ranges are diagnostic: either finite ordered endpoints or
the explicit full unlimited range. One-sided infinity is rejected. Min/max
retain original Java signed-zero, infinity and NaN behavior; they do not clamp
finite samples or provide bounds for arbitrary callbacks. Typed callback errors,
exhausted budget and nonfinite coordinates/results stop immediately. A nonfinite
child stops before remaining right callbacks, whereas Java can continue to a
nonfinite root. This checked error policy is a deliberate `FerrumC` deviation;
prior callback effects are not rolled back and graph data remain unchanged.

The separate original-JAR corpus covers 38 synthetic graphs, 7,588 finite sample
bits/traces, 353 nonfinite results, 45 source-exception traces and original range
diagnostics. It includes adjacent floats, signed zeros, subnormals, overflow,
depth 32 and repeated-child DAGs. It copies no terrain profiles. Exact pins,
reproduction and limitations are in
[GEN-04.3](../../../docs/agent-tasks/GEN-04_3_CUBIC_SPLINES.md).
Codecs, builders with value transformers, coordinate visitors, registered terrain
profiles and f64 density-coordinate adapters remain consumer work. This module
adds no generator identity and activates no natural chunks.

## Block-entity storage codec

`try_encode_block_entity` appends a versioned payload only after the full entity
has been validated and encoded. Schema 1 starts with `0x7f`, a big-endian `u16`
version, and the entity type. Chest bodies hold the fixed 27 slots as
`u32`-length-framed records from the independent `ferrumc-items` persistence
codec. Supported component variants, opaque payloads, removals, NBT order,
duplicate NBT names, and exact float bits survive round trips.

The entire entity has a 16 KiB encoded-byte cap. All chest slots share this
aggregate allowance; a caller cannot reserve each slot's maximum separately.
Decoding borrows bounded complete child records after checking the parent input,
and any malformed child rejects the entire entity. Checked encoding retains
only one temporary item record at a time and leaves the caller's buffer unchanged
on any failure. Sign fields are likewise bounded without truncating UTF-8.

`decode_block_entity` also reads the old unversioned grammar explicitly. Legacy
chest records migrate to componentless stacks with their original item/count;
old records cannot recover components that their writer never stored.
The production App/Sim/Storage path now uses the checked writer and checked
overlay capture/application. A failed capture retains the selected dirty
batch; a failed apply leaves its target unchanged. Storage also bounds the
combined encoded entity tail to 2 MiB and each full-chunk/overlay record to
3 MiB. These are encoded-byte caps, not decoded-heap or RSS guarantees.

`encode_block_entity` retains the legacy, lossy writer for existing callers.
The experimental multi-shard scheduler still uses that legacy capture path;
it is not the production App path. Crash-atomic updates across player,
overlay and journal records remain separate work.

## Fixed builtin biome temperature

`biome_temperature::BuiltinBiomeTemperatures` compiles the complete65-biome
climate table and five fixed Simplex objects into an immutable2232-byte value.
Construction/clone/query allocate nothing. Queries use checked biome IDs and
all signed block/sea-level bits, preserving original f32 operation order and
pure Paper rain/snow/iceberg predicates. No world seed or mutable cache enters
this producer, and no surface or snow/ice world mutation is activated.

The [temperature reference](tests/reference/biome-temperature/README.md) retains
all129588 genuine original labelled cases per mode, direct/fixed constructor
and complete Frozen-grid evidence, explicit Vanilla cache/Paper offset
differences, parser boundary tests and external allocation measurements.

## Original overworld surface rules

`surface_rules` compiles the original overworld `surface_rule` of the pinned
Registry export (428 nodes; `large_biomes` and `amplified` are identical) into
an immutable `SurfaceRuleTemplate` and binds it per seed as
`SeededSurfaceRules`: Paper build 60 numerics only (the second consumer of the
Paper `ImprovedNoise` basis next to the `NoiseChunk` session), Paper zoom
semantics through the GEN-05.8 resolver and the Paper default flat-bedrock
mode. `SurfaceRuleSession` evaluates one default-block position at a time
against explicit corners, column, block inputs and a borrowed heights view,
with typed refusals for policy domains, budgets and mismatched resolvers.
Session calls do not allocate.

The [surface-rule reference](tests/reference/surface-rules/README.md) holds
the frozen original helper, runner, guards, projection and fixture generator,
the Root-authorized clay extension (A5) and the allocation runner. Tests
replay all 38,889 fixture rows under the declared `full`/`partial`/`refused`
classes with 720 actual GEN-05.5 arrays. The column driver (stone depth, water
height, bandlands/iceberg extras and block writes) is the GEN-05.10 submodule
below; no surface is activated, and no performance claim is made.

## Original overworld surface column driver

`surface_rules::column` (re-exported from `surface_rules`) is the original
`SurfaceSystem.buildSurface` of the overworld SURFACE step for one centre
chunk. `SeededSurfaceColumns::bind` derives, from a `SeededSurfaceRules`
binding, the six extension noises (`badlands_*`, `iceberg_*`, Paper basis) and
the closed 35-state alphabet (GEN-05.4 material states, rule results, clay
bands, `snow_block`, `packed_ice`), classified by block name.
`build_chunk` copies the GEN-05.4 material into a caller-owned
`SurfaceChunkBuffer`, visits the columns in the original order with one fresh
GEN-05.9 session per chunk, runs the eroded-badlands prepass and the
frozen-ocean postpass with the original randoms, keeps `WORLD_SURFACE_WG` live
for every `evaluate` and publishes all or nothing (states, heights, fluid
flags). It refuses inconsistent bindings and materials, the session's resolver
checks and a driver allowance below `SURFACE_CHUNK_WORK` before any column
work, and does not allocate. The preliminary corners are caller-trusted.

The [column reference](tests/reference/surface-column/README.md) holds the
frozen GEN-05.10 helper, runner, guards, projection and fixture generator and
the allocation runner. Tests replay all 70 admitted fixture chunks (17,920
columns, 2,261,637 rule calls) from the genuine producers against the
projected original trace, final digest, heights and fluid marks; the two
refused groups expect only their typed producer or session refusal.
`OCEAN_FLOOR_WG`, carvers, features, lighting, persistence and activation are
out of scope; no performance claim is made.

## Invariants

See `INVARIANTS.md` in this directory.

## Cold original overworld Carvers

`carver::SeededCarvers` binds the closed overworld alphabet and original
replaceability tag. `prepare_run` admits identities, the private source halo,
native capacities and cumulative producer work before creating fresh Paper
Noise, Aquifer and selector state. `carve_chunk` consumes that run once and
publishes an independent `CarvedChunkBuffer` only on complete success. Every
refusal first empties the output; the input Surface remains Published and
unchanged. Preliminary corners retain the caller-trusted provenance contract.

Every one of the 289 source queries executes in X-outer/Z-inner order before
its constant original carver list is used. Actual lazy fix-ups interleave with
that history through the same surface-rule evaluator as Stored evaluation.
Only a biome memo miss calls the concrete private `DirectCarver` provider;
temperature shares that memo. Noise caches, selector history and Aquifer caches
and stale Lava flags persist through the whole run. Each fix-up refreshes only
the rule column/block context. The private derived halo and quart Y −17 are
unclamped; public coordinate and budget domains retain their existing caps.

Noise and selector work use a private checked three-u64 cumulative meter.
Bounded segments are bookkeeping; they cannot reset producers or narrow a
fault. Public counters and errors retain their public contracts. The runtime
preserves original Legacy geometry RNG, f32 arithmetic, SIN rawbits, mask and
write order, and live `WORLD_SURFACE_WG` rescans. The SIN table is an immutable
262144-byte static imported mechanically from all 65536 genuine original bits.
Fluid output is a declared set policy; the fixture retains original ordered,
duplicate postprocessing evidence for later consumers.

The [Carver reference](tests/reference/carver-runtime/README.md) describes the
full producer replay, labelled controls, native layouts and cold allocator
checks. The very large finite Aquifer RNG period bound is counter accounting,
not a timing guarantee. Original unmetered Legacy geometry rejections and
shuffling during earlier immutable router binding remain explicitly scoped
policies. This producer does not activate natural-world generation, warm
thread history, features, persistence, lighting, or App behavior.


## Natural Overworld terrain preview

`NaturalWorldGenerator::new(WorldSeed)` binds the accepted normal Overworld
producers. `try_generate_chunk(ChunkPos)` composes real material terrain,
preliminary surface corners, a complete cold stored-biome neighborhood, original
Paper build-60 surface columns and carvers. Every producer must succeed before
a live `Chunk` is returned. Refusals keep their producer phase and diagnostic in
`WorldError`; there is no synthetic terrain or flat fallback.

`RuntimeWorldGenerator::flat(seed)` and `natural(seed)` select the configured
algorithm. Both use the object-safe `FallibleChunkGenerator` contract, while the
legacy infallible `ChunkGenerator` remains available for existing flat callers.
Clones share immutable seed bindings. Callers must run CPU work on a bounded
generation worker lane; World creates no threads and performs no I/O.

Natural chunks retain actual quart biomes. Configuration must send
`network_biome_resources()` in exactly the order used by the network encoder:
plains first, then the remaining 64 builtin resources in canonical-name order.
Mixed sections use the client-compatible direct 7-bit biome representation.
`ChunkLightData::vertical_sky` supplies conservative sky light for vertically
open air; it leaves enclosed caves dark, without implementing full propagation.

This versioned preview includes material-stage ore veins but omits placed-feature
ore generation, trees, other vegetation, structures, fluid simulation and full
lighting. Its public installation tests compare every FINAL block of the accepted
original seam and signed-seed fixtures. It is terrain preview, not a complete
Survival world or a live-Paper server acceptance claim.
