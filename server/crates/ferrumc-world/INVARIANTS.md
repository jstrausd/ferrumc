# Invariants: ferrumc-world

> Rules that must hold for all code in this crate. Violating these is a bug.

## Original biome selection and complete biome arrays

- `BiomeTree` consumes only checked builtin ordered parameter resources. It
  retains every repeated entry, exact stable seven-axis RTree structure and
  original leaf ordinal. Original strict comparisons and Java wrapping i64
  distance arithmetic apply to the entire quantized i64 target domain; there
  is no linear search or alternate tie resolver. Tree data are immutable Arc-
  shared, while selection history and cumulative work are exclusively owned.
- Construction reserves a metered exact count-only original bucket shape
  before allocation. Separate caps are8192 parameters,10000 nodes,9999 edges,
  depth8,2MiB retained native capacity and1MiB simultaneous scratch. Fallible
  reservations and before-growth checks discard every failed candidate. Build
  allowance is1..=67108864; queries share1..=268435456 until explicit reset.
- Source identity is explicit multinoise, one checked fixed resource or the
  original five checked End resources. Noise settings do not choose a source.
  Moving a scalar session into a source transfers its exact history, status and
  cumulative meter. Fixed and End never acquire a multinoise leaf history;
  fixed performs no climate work. Source reset is allocation-free and returns
  live/cold/zero-work without changing identity or silently resetting noise.
- `BiomeQuartGeometry` is a full independent height accessor:1..32 complete
  sections, aligned minimum, checked internal i32 Y. It must contain the entire
  admitted noise interval; accessors requiring original forChunk height
  clamping are refused before work/state mutation. Full Nether/End arrays
  cover0/256 although their original noise settings cover0/128.
- Execution order is ascending section-Y, local-X, local-Y, local-Z. Storage is
  ascending quart-Y/X/Z. At most2048 values and8192 simultaneously staged/final
  native capacity bytes are admitted fallibly before allocation. No per-quart
  allocation, runtime parser, IO, wire ID or partial output is introduced.
- A noise session admits one biome phase only while its material/density phase
  is Fresh. Successful biomes retain Fresh material authority; failed execution
  poisons noise and source. Multinoise history commits once after complete
  publication; failure retains caller-visible pre-array history. Admission
  failure is mutation-free. Each session must reset explicitly; resetting one
  never heals the other. Source work stays cumulative over different chunks.
- End preserves the original checked section-center transform and i64 radius
  test<=4096, then uses cached erosion f64 at the transformed center. Thresholds
  are>0.25,>=-0.0625,<-0.21875 in that order. Existing noise finite-value refusal
  applies; no f32 climate substitution or fallback biome is allowed. The local
  ±30M transform admission rejects qX7500000 (center30000008), admits qX7499999
  and qX-7500000, and preserves negative arithmetic floors. This is FerrumC
  admission policy, not a claim to Java's coordinate-wrap/NaN behavior.
- Complete original tree/scalar evidence is adopted unchanged. New156 whole
  arrays execute actual original createBiomes, empty blending and the real
  StructureManager/Beardifier reading only original empty references through a
  fail-closed LevelAccessor. Instrumentation delegates each original source/
  root once and verifies24 unwrapped generator baselines.27 original End probes
  independently establish exact radius/threshold equality. Numerical compute
  acceptance does not install chunks, client palettes, persistence histories,
  surface/carvers/features/structures, scheduling, Survival or Java plugins.

## General

- No `unwrap()` or `expect()` outside `#[cfg(test)]`.
- No unbounded channels or allocations from untrusted input.
- All public items have rustdoc.
- Error types classify the failure mode.

## Crate-Specific

- `BlockStateId::AIR` is raw `0` and is the `Default`; "empty" everywhere means air.
- `PackedArray` is non-spanning: each `u64` holds `floor(64 / bits_per_entry)`
  whole entries, low bits first; an entry never crosses a word boundary.
- `PackedArray::new` accepts only `bits_per_entry` in `1..=64`.
- `PalettedContainer` indices and ids are validated before storage: an indirect
  palette index is always `< palette.len()`, and the packed width is always wide
  enough to hold every live index.
- `PalettedContainer::non_air_count` is maintained incrementally and always
  equals the true number of non-air entries (so `air_count == CAPACITY - non_air`).
- Representation only ever promotes (`Single` → `Indirect` → `Direct`); it never
  demotes, so reads stay cheap and counts stay consistent.
- A `LocalBlockPos` index is always in `0..4096`, so `ChunkSection` get/set are
  total functions that never error for any valid position.
- A `Chunk` has exactly `SECTION_COUNT` (`HEIGHT / 16 = 24`) sections, stacked
  bottom-to-top with `sections[0]` at `MIN_Y`. World `y` maps to section
  `(y - MIN_Y) / 16` and local `y` `(y - MIN_Y) % 16`; this floors correctly for
  negative `y` (`y = -64` → section 0, `y = -48` → section 1).
- `Chunk::get_block`/`set_block` take an absolute `BlockPos` and reject any
  position outside the chunk's own column or the buildable height range without
  panicking (`None` / `WorldError::BlockOutsideChunk`).
- `set_block` marks a section dirty only when the stored value actually changes;
  `DirtySections` ignores out-of-range indices and never panics.
- A `Heightmap` entry is the world `y` of the highest non-air block in a column,
  or `None` for an all-air column. For the current (all-solid) block set the
  motion-blocking and world-surface heightmaps are identical.
- `FlatWorldGenerator` is deterministic: the same `ChunkPos` always yields an
  equal `Chunk`, and the per-column profile is identical for every column and
  every chunk. Layer layout: `y = -64` bedrock, `-63..=59` stone, `60..=62`
  dirt, `63` grass, `64..=319` air.

## Generator identity contract

- `WorldSeed` preserves the entire signed `i64` domain. This crate never
  chooses a random seed, hashes a textual seed, or persists world metadata.
- `GenerationIdentity` is a closed supported value: Minecraft 1.21.8,
  `FerrumcFlat` revision 1, Overworld geometry, and an explicit numeric seed.
  Unknown releases, algorithms, revisions, and dimensions fail decoding and
  are never treated as the current generator. No additional terrain algorithm
  is available through this contract.
- Its schema-1 `FCGI` envelope is exactly 27 bytes: magic (4), big-endian schema
  (u16), Minecraft major/minor/patch (three u16), algorithm (u16), algorithm
  revision (u32), signed seed (i64), dimension (u8). Algorithm 1 is FerrumC flat;
  dimension 0 is Overworld. Length is checked before any field access. Encoding
  returns a fixed array and decoding performs no allocations.
- Envelope schema describes metadata layout. Generator revision describes the
  generated baseline. Changing flat layers, registry baseline, dimension
  geometry, palette generation order or generated dirty-mask semantics needs an
  explicit identity/version decision; changing envelope layout is independent.
- Compatibility requires full identity equality, including seed. With the
  current closed set, only seed changes can compare as two known incompatible
  values; other unsupported metadata is rejected before comparison.
- `ChunkGenerator` implementations are pure, synchronous and request-order
  independent. A given identity and `ChunkPos` must yield the same blocks,
  palette representation and dirty masks. The identity stays constant for the
  lifetime of the generator. No storage/network handles or I/O enter this lane.
- The legacy `FlatWorldGenerator::new/default` selects numeric seed 0;
  `with_seed` selects another identity without changing seed-independent flat
  blocks. Existing `generate` calls use the same implementation as
  `ChunkGenerator::generate_chunk`. Generation marks network dirty sections
  0..7 and does not mark gameplay-persist dirty/edited sections.
- Storage metadata, overlay compatibility gates and configured numeric seed
  selection belong to the separately implemented App/Sim/Storage consumers.
  This pure contract alone does not protect restart or migrate existing worlds.
  Multiple dimensions and natural terrain remain separate consumer work.

## Original random and noise primitives

- Random streams are exclusively owned and preserve all signed seed bits.
  Explicit legacy/xoroshiro selection, numeric seed upgrade, zero-state fallback,
  signed legacy `nextLong` summands, Java int-before-long coordinate overflow,
  UTF16 name hashing, MD5 UTF-8 seed halves, and parent fork consumption follow
  the pinned Minecraft 1.21.8 algorithms. Copies are independent snapshots.
- Positional factories are immutable; coordinate/name/seed request order does
  not affect derived streams. Legacy numeric derivation ignores factory seed;
  xoroshiro numeric derivation XORs both halves, as in the original.
- Invalid signed bounded-int requests and excessive explicit consumption fail
  before state mutation. Consume count zero is no-op; the explicit per-call CPU
  bound is 1,048,576. Gaussian and Java isolated-surrogate strings are not modeled.
- Improved noise owns exactly 256 permutation entries and three original
  seeded offsets. Sampling and analytic derivative evaluation allocate nothing
  and do not consume random output. Vertical quantization changes gradient y,
  while the y interpolation weight remains unshifted.
- Perlin configuration validates before allocation or RNG consumption: 1..64
  stored amplitudes, first/last octave in -64..32, finite amplitudes and a finite
  calculated maximum. Legacy positive octaves fail. These resource/numeric
  restrictions and earlier error timing are explicit differences from Java.
- Modern initialization consumes one positional fork and uses `octave_N`
  name-derived noise; zero amplitudes retain gaps. Legacy always constructs
  initial octave zero and consumes exactly 262 next-int steps per skipped
  lower octave. Gap skipping is not substituted with noise construction.
- Public sampling/wrap coordinates and vertical parameters must be finite and
  at most 2,147,483,136 in magnitude. Nonfinite calculated results fail rather
  than becoming a terrain fallback. Octave wrapping and amplitude accumulation
  preserve original evaluation order; no deliberate FMA or reassociation.
- Goldens come from direct reflection calls into the hash-pinned original JAR,
  with pinned official mappings and bundled library hashes. Comparisons use
  exact integers and float bits. They establish the tested primitive cases on
  the documented toolchain, not complete world-seed or cross-platform parity.
- No seeded density/router sampling, chunk consumer, Natural-world identity,
  persistence gate, event, storage/thread handle or Gaussian API is activated
  by these primitives. The separate compiler admits metadata and graphs; seeded point sampling is
  provided by its explicitly bounded router consumer.
- Simplex retains only the original 256 addressed permutation values. Its
  original extra 256 zero entries are unused. Construction shares the identical
  Improved shuffle; seeded offsets are not added by the original 2D/3D samplers.
  Strict/equal ordering branches, saturating double-to-int floor casts and
  wrapping i32 floor sums are preserved, including distant coordinates.
- Normal constructs two independently initialized Perlin stacks. Original
  amplitude-span normalization includes Java's overflowing sentinel subtraction
  for all-zero lists. It validates both stack maxima and their combined maximum
  before allocation or RNG consumption; signed amplitudes/zeros are retained.
  Its explicit caller cap is `MAX_NOISE_COORDINATE / 1.0181268882175227` and the
  second scaled coordinates are independently checked before either stack runs.
- Blended parameters are private validated values in the original codec domain:
  four finite factors in 0.001..1000, smear in 1..8. This deliberately excludes
  out-of-codec inputs accepted by the original Java testing constructor.
  The 16/16/8 legacy stacks are reconstructed by `with_new_random` using only
  retained parameters; old stacks and the original parent are never mutated.
  Public sampling accepts every i32 `BlockPos`; internal wrapped coordinates
  and unwrapped yMax bypass public Improved/Perlin caller caps. Original
  8-/16-iteration order, division, clamp flags and strict clamped-lerp boundaries
  are preserved. Constructor errors leave the supplied RNG unchanged.
- Composite-noise fixtures are separate from GEN-04.1. They call actual Normal,
  Simplex, Blended, and Mth original-JAR methods after original version detection
  and registry bootstrap; the oracle contains no copied numerical noise formula.
  Public sampling is immutable, allocation-free, synchronous and free of I/O.

## Original cubic splines

- `CubicSpline` retains only owned immutable graph data. Borrowed specifications
  and point/range slices are not retained. Clones share immutable `Arc` data,
  carry no sampling cache and remain valid after the source or original clone
  is dropped. IDs address construction-local slice indices, not persistent or
  globally unique handles; equal numbers cannot prove graph provenance.
- Point/constant inputs are finite private-field values. Points per multipoint
  are 1..=64 and strictly ascending; duplicate locations include both zeros.
  Coordinate diagnostics are finite ordered pairs or explicit full unlimited
  ranges. Reversed, NaN and one-sided-infinite ranges fail construction.
- Before owned graph allocation, all specifications, including unreachable
  nodes, satisfy the 1024-node, 64-coordinate, 64-point/node, 16,384-total-point
  caps and valid root/child/coordinate indices. Fixed-capacity iterative DFS
  rejects even long cycles before checking depth. Maximum DAG depth is 32,
  counting a constant leaf as one. Failure releases no partial public graph.
- Sampling accepts a caller budget in 1..=1,048,576. A node entry, coordinate
  callback or binary-search comparison consumes one unit before that operation;
  budget admission fails before callbacks. Shared subgraphs can require
  exponential revisits, so depth alone never substitutes for this work cap.
  Callback work and its allocations remain the caller's responsibility.
- Binsearch finds the first location strictly greater than the sample; equality
  enters the interval on its right. Lower/upper extensions evaluate only the
  endpoint child, and a signed-zero derivative returns that child unchanged.
  Interiors evaluate the left child before the right even at an exact interior
  endpoint. Shared child/coordinate requests repeat without memoization.
- Interpolation, extrapolation and diagnostics preserve original f32 order;
  no deliberate FMA, algebraic reassociation or f64 interpolation. A future
  density adapter casts original f64 coordinates to f32 before sampling
  and widens the f32 result after sampling, as the original does.
- Original Java min/max propagate NaN and preserve signed-zero distinctions.
  Diagnostic overflow/infinity/NaN is retained and never guards finite sample
  coordinates. Reported coordinate ranges neither clamp callback values nor
  bound arbitrary callback behavior.
- Actual coordinate values and every child/root sample result must be finite.
  Typed source failure, budget exhaustion or nonfinite value stops immediately;
  this checked policy is additional FerrumC behavior. Nonfinite child errors
  can skip right callbacks that Java would execute. Only successful finite
  samples claim original bit and request-order parity. Prior callback effects
  are not rolled back; the immutable graph is unchanged by every sample error.
- The spline corpus calls hash-pinned original `CubicSpline` methods through
  reflection and real `ToFloatFunction` proxies. Finite arithmetic, diagnostics
  and source-error traces are compared separately from Rust nonfinite policy.
  It supplies no copied `TerrainProvider` profiles, density/registry/router,
  codec/visitor, aquifer, surface, biome or natural-chunk implementation.

## Original builtin density compiler

- `DensityTemplate::compile_definition` and `RouterTemplate::compile_builtin`
  admit only typed pinned Registry resources. All 35 density definitions,
  including unused definitions, are validated before a template is returned.
  No raw JSON compiler, unchecked node handle, default router, natural-world
  identity or chunk output is exposed. The explicit router seed-binding API
  supplies the separately checked scalar/climate consumer below.
- Graphs are owned immutable `Arc` data. Every original density helper input,
  including inline constants/objects, retains its `HolderHolder`; raw spline
  coordinate holders remain separate vertices. Named targets are resolved only
  after every definition has been parsed. Joint cycle/depth checks include all
  density, spline, coordinate and holder edges and unreachable nodes.
- Preflight charges each distinct density/settings/noise original source before
  parsing, with an aggregate 2 MiB cap. Per-source bounds are 128 KiB, 4096 JSON
  values/keys, depth 32 (root zero), 256 container entries and 256 decoded UTF-8
  bytes per key/string. The whole source bounds temporary escape-decoder memory.
- Private IR appends check 4096 vertices/8192 edges before growth. Combined
  depth is at most 96 (leaf one); the actual seven-router codec union measures
  3102/3435/depth87. Original spline-local limits also apply; they never replace
  the joint cycle/depth check. No partial public graph escapes on failure.
- The bounded JSON lexer preserves original number tokens and full grammar,
  rejects duplicate decoded keys/trailing input and delegates only string escape
  decoding. DOUBLE and FLOAT decode directly to their respective bit widths.
  INT preserves original Gson truncation/wrapping before range validation,
  including its 10000-character token cap and absolute decimal scale <10000.
- Spline derivative is required by the actual codec. Unknown fields, nonfinite
  DOUBLE/FLOAT values and nonascending spline locations are explicit additional
  checked admission policy. Original codecs permit reversed clamp/range and
  equal/reversed gradient endpoints; compilation does too.
- Noise inputs are named pinned builtin references. Original codec inline noise
  parameter holders are outside this closed public builtin compiler scope and
  are rejected by private admission rather than conflated with named resources.
- Producer byte/node/depth and compiled dependency categories are independently
  checked. Original default block/fluid, surface and spawn payloads stay bounded
  deferred static slices; material/surface/spawn execution remains absent.
- Actual original-JAR codec projections verify all 35 definitions, seven routers,
  raw bits, holder identities and 32 node forms; six direct factory probes are
  separate evidence for later visitor lowering. Structural acceptance supplies
  no density sampling, worldgen, Survival or Paper-runtime parity.

## Original seeded density and climate consumer

- Seed binding owns one immutable graph/noise pool and two original visitor
  views. It preserves every signed64 seed, selected algorithm, canonical named
  positional streams, legacy temperature/vegetation wrapping seeds and zero
  offset special case, independent Blended terrain seed and End 17292-step consumption.
  Private aquifer/ore positional factories are retained; no material consumer
  is activated. Binding errors cannot publish a partial router or mutate a
  previously returned router/template/clone.
- Decoded JSON objects and actual registered native `NoiseRouterData` objects
  have distinct origins. Decoded inline HolderHolder stays; native inline
  codec artifacts are omitted. Reference holders and raw spline coordinates
  resolve to the registered graph. Sparse native variants share unchanged
  immutable nodes; each tested settings graph needs 162 additional vertices.
  Both prepared/visitor views keep 4096 vertices, 8192 edges and depth 96 limits, with
  pre-growth vertex/edge checks and the complete existing graph validator.
- Actual factories, source indices and holder origins are retained privately.
  The seed visitor preserves Holder/Marker factories and rebuilds original
  diagnostics. The climate visitor removes them without compute charges and
  rebuilds their ancestors. Native and already-existing MulOrAdd uses its own
  mapAll bounds; decoded Ap2 uses the original two-input factory path. Spline
  diagnostics use the existing original f32 builder, with a private coordinate
  range adapter that preserves arbitrary original diagnostics while public
  caller admission stays unchanged.
- Bounds preserve exact original signed zero, reversed, infinity and NaN values.
  They are not guaranteed mathematical hulls and never clamp samples. Java
  min/max and original mapped diagnostics are preserved. Actual samples use
  the original evaluation/shortcircuit order; Ap2 mul ±0 skips right and yields
  +0, whereas MulOrAdd with a zero argument evaluates input. MIN/MAX skips use strict right bounds.
  RangeChoice selects exactly one child; spline keeps low-before-high order.
- One work meter crosses every density/spline/noise transition. Charges occur
  before executed density/spline entries, coordinate callbacks, binary-search
  comparisons, sampler invocations, every fixed octave slot including zeros
  and every one of 625 End candidates plus eligible Simplexinvocations. Removed
  climate holders/markers are aliases and uncharged. Exhaustion stops the next
  operation; there is no recursion-local allowance reset, sample value cache,
  mutable RNG use, I/O, lock or production point/climate allocation.
- Point and six-axis climate budget is 1..=1,048,576. Batch shares one budget of
  1..=67,108,864 over 1..=4096 ordered requests. All shape/position checks happen
  before sampling and bounded output allocation. First failure stops later
  requests; no partial output is published. Errors identify source, target,
  position, batch index where applicable and limit/used or raw nonfinite bits.
- Block X/Z include ±30,000,000; Y includes all i32. Quart-to-block conversion
  checks i64 products before casts and refuses shift wrap. from_block uses
  arithmetic >>2 including negative unaligned coordinates. sample_climate uses
  quart origin; sample_climate_at_block uses the exact block. Six original
  f64 results cast to f32 then quantize with f32 × 10000 and Java-long saturation.
  Neither method resolves biomes, spawn or chunk terrain.
- Legitimate 45 billion scaled jagged inputs use private finite checked per-octave wrap,
  including original Normal second scaling and long-floor conversion checks.
  Public noise caller caps stay unchanged. Every actually requested completed
  density/spline node and f32coordinate must be finite. Checked refusal can
  stop before a later Java child even when Java's final parent is finite; no
  numerical fallback or diagnostics normalization is supplied.
- Plain End density follows Vanilla truncating division/remainder and initial
  wrapping i32 radius, preserving full 625 candidate evaluation before checked
  nonfinite refusal. Paper widens that radius and has a cache; Vanilla goldens
  do not establish Paper End parity or authorize its terrain activation.
- The new actual-JAR corpus separately proves both recursive visitor factory
  views, scalar/bounds/climate bits, registered origins, wrapped noise, named
  seed controls, positional factory draws and original branch traces. Rust
  tests prove its own budgets, finite policy, admission and atomic publication;
  Java does not implement or prove FerrumC work budgets. Existing primitive,
  composite-noise, public spline and seed-free codec corpora stay unchanged.

## Fresh Paper NoiseChunk consumer

- `SeededRouter::fresh_paper_noise_chunk` creates an exclusively owned session
  for one full settings height and 16×16 chunk, with empty blending, no structures,
  retrogen or height-accessor clamp. Immutable router/noise Arcs remain unchanged;
  every mutable wrapper, array, preliminary cache and epoch belongs to the session.
- Public scalar/noise/Climate consumers retain Vanilla reference behavior.
  Only this Paper Build60 session privately copies Improved offsets through f32
  with unchanged permutations/draw order, and uses the widened i64 End radius.
  No random streams are sampled during runtime density evaluation.
- Original seed/mapAll/wrap chronology, record-equality partition, actual
  identity-bearing holder/noise/spline-array keys and mutable registration order
  are retained. All fifteen roots are wrapped before material-helper wrapping;
  cached Climate roots reuse the same visitor map and cache identities.
- All graph/child/type/depth checks and resource admissions precede publication:
  4096 vertices,8192 edges,depth96;4096 equality/map/identity entries;131072 cold
  visits;128 wrappers;2MiB key metadata;2MiB retained graph metadata;4MiB cold
  temporary metadata;8MiB cache arrays;4MiB simultaneous scratch. These measure
  owned element bytes with checked arithmetic, not process RSS or a performance
  promise. Existing immutable source Arcs are accounted separately.
- One budget in 1..=268435456 crosses constructor, graph/equality probes,
  density/spline/noise, cache writes, context transitions, array copies, driver
  operations, output writes and preliminary-cache comparisons until reset.
  Exhaustion stops before the next operation. Cache hits never bypass metering.
- A session runs Fresh→Running→Complete exactly once. Internal driver phases
  admit initialization, first/advanced slices, selected cell, Y→X→Z updates,
  swap and stop in that order; advancing invalidates selected-cell state.
  Illegal lifecycle, column, shape, index and nonfinite-delta requests reject
  before changing caches, counters or work. Full private geometry is admitted
  before buffers and coordinate multiplication.
- Five wrapper paths preserve distinct own/foreign semantics. Cache2D initially
  hits its actual sentinel with +0 and ignores Y. CacheOnce checks array epoch
  and logical length before scalar epoch; fixed reserved capacity is never a
  valid length. Failure publishes the original key/counter timing but no value
  or successful array epoch. Flatcache includes its 5×5 grid; outside reads
  delegate. Cell cache indexes descending-Y/X/Z only for owned active contexts.
- Slice-forIndex increments the scalar epoch and exposes index i. Cell-forIndex
  leaves the scalar epoch unchanged. Cell direct-fill exposes i+1 to compute
  because Java evaluates arrayIndex++ before its RHS; the index128 CacheOnce
  refusal is preserved. Interpolator cell-fill uses original X→Y→Z lerp3;
  normal updates use Y→X→Z without reassociation or deliberate FMA.
- Array dispatch preserves ADD left-all/right-all, MUL ±0 skip to +0,
  MulOrAdd input evaluation even with zero, strict MIN/MAX bounds, selected
  RangeChoice child and context-transformer provider requests. Scratch leases
  return on every error. Nonfinite completed children/results fail immediately;
  Java's subsequent NaN requests are not a finite request-parity claim.
- Complete material-density output is ascending-Y/X/Z, exactly256×height finite
  f64s. A separately bounded bitmap proves every destination is written once;
  no partial output escapes. No aquifer, material block/fluid, biome, surface,
  chunk storage or natural-generator identity is selected by these samples.
- Horizontal caller/corner positions include ±30000000; Y includes all i32.
  Preliminary level ignores Y, aligns X/Z to quarts, compares strict0.390625
  from top through minY, and returns i32::MAX when none passes. Its2048-entry
  cache admits hits even when full, never evicts, and publishes only success.
  Cached Climate computes six roots in original order, casts f64→f32 before
  quantization and reports typed axis failure on finite-double/f32 overflow.
- Executed public query errors poison the session; rejected admission/lifecycle
  requests leave it unchanged. Reset constructs and validates a complete new
  candidate before swapping; a failed reset preserves every prior cache,
  counter, lifecycle and used-work value. Successful owned outputs remain valid
  after their source session/router is dropped.
- Actual-JAR complete bits, constructor equality/sharing, wrapper/provider/
  array traces, diagnostic arrays, Surface/Climate and End seams are tested
  independently from Rust-only caps, budgets, poison/reset and publication.
  These establish the tested numerical context, not a playable Survival server,
  complete natural generation, Java-plugin compatibility or RAM/TPS improvement.

## Fresh Paper material-array consumer

- `sample_chunk_materials` shares the single Fresh/Running/Complete/Poisoned
  authority with `sample_chunk_densities`; neither can run after the other
  without successful reset. Its public output owns seed/settings/chunk/geometry,
  at most131072 block states (512KiB), and a separately admitted fluid bitmap
  (16KiB), in the identical ascending-Y/X/Z output order. A separate16KiB
  exactly-once bitmap guards writes; it never doubles as fluid state.
- One concrete private Sinkenum uses the existing complete interpolation loop.
  Material density enters only at Running/ZUpdated, then aquifer→optional ore→
  settingsdefault is evaluated in original order. Final postprocessing requires
  the actual aquiferflag and selected non-air state with nonempty fluid.
  Density arrays retain their existing numerical/request/work contracts.
- Material preparation occurs only at its Fresh entry, before buffers/driver.
  Original aquifer constructor and ore closure draw nothing; center/status/RNG
  work stays lazy at actual branches. Preparation failures poison the admitted
  run; rejected lifecycle requests leave it unchanged. No partial output escapes.
  Atomic reset preserves old state on failure and publishes a new unprepared
  Fresh session only after complete .3construction.
- Original aquifer grid/index shape owns matching location/status arrays:
  at most512 slots/12KiB, actual three enabled settings315slots. Long.MAX_VALUE
  is the original location sentinel; status is absent until successful publish.
  Four nearest centers preserve twelve X/Y/Z requests and >= tie chronology.
  Pressure shares one original Barrier sentinel; skipped Density/RNG branches
  are never evaluated in advance. Every actual bounded-int rejection draw is
  charged before RNG mutation; all material work uses the same session meter.
- Settingsstate resolver checks≤4096 sourcebytes and≤32properties through the
  existing bounded JSONlexer and complete pinned Registry lookup. Default
  inheritance is preserved; unknown fields/names/properties/values are typed
  refusal. This is stricter malformed-input policy than Java's lenient optional
  properties, not claimed error-input parity. Actual seven defaults plus fixed
  aquifer/vein states form a closed fluid/state set; no AIRfallback or solidity-
  based fluid inference is permitted.
- F32thresholds widen from their actual f32bits in Double expressions. Global
  fluid, original strict height/pressure/ore tests, quantize/map order and
  wrapping MAXpreliminary+8 arithmetic are preserved. Requested nonfinite
  Density results or material arithmetic refuse with rawbit classified failure.
- The material-only physical halo is±30000128. Derived center displacement is
  X/Z[-20,+20],Y[-22,+21]; original thirteen offsets and quartfloor produce
  surfaceX[-71,+36],Z[-39,+36]. Before buffers/driver, i64derived extremes and
  packed Center-Y in[-2048,+2047] are admitted. Foreign Spread addresses divide
  physical X/Z by16 and Y by40; Lava divides X/Z by64/Yby40. Their separately
  admitted scaled domains are not relative chunk-neighbor coordinates. Public
  scalar/Climate/preliminary domains remain unchanged at±30000000.
- Caches live only for this complete material-array run and are released with
  the private sink; no public Carverreuse, surface/carver/feature/structure,
  lighting, biome adoption, live natural-world identity, gameplay or plugin
  compatibility follows. Full actual-Paper materialarrays and independent
  branch/guard/budget acceptance are distinct from active server acceptance.
- Bounded declared test leaves, exact point/status inputs, contexts and RNG
  traces exist only under `cfg(test)`; production keeps concrete factories and
  global-fluid logic without callbacks or per-block trace allocation. Original
  enabled/cache evidence compares occupied arrays and signed-packed Preliminary
  keys after every call, including repeats. Fifteen original nonfinite probes
  verify typed raw-bit Rust refusal and failed cache nonpublication, not Java
  successful-result parity.

## Block-entity persistence

- `try_encode_block_entity` is the checked, versioned storage writer. Its
  envelope is discriminator `0x7f`, big-endian `u16` schema 1, entity tag, then
  the entity body. Schema is independent of parent overlay metadata.
- Chest bodies contain exactly 27 `u32`-length-framed item records from
  `ferrumc-items`' lossless persistence codec. The slot count comes from the
  model, never an untrusted count. Supported component representations, patch
  order, removed ids, and NBT details are preserved without normalization.
- The whole encoded entity is at most 16 KiB, including every slot and all
  length prefixes. This is an aggregate encoded-byte cap, not the per-item cap
  multiplied by the number of slots, and not a decoded-heap allowance.
- Decode checks the whole parent slice before allocations; slot lengths are
  checked before borrowing complete child records. A malformed child rejects
  the entire entity, with its fixed slot index and classified item error.
- Checked encoding keeps one bounded temporary item buffer at a time and builds
  an entity privately. Any count, component, NBT, sign-field, or aggregate-limit
  error leaves the caller's output buffer completely unchanged.
- Decode explicitly migrates old unversioned sign/chest payloads. Legacy chest
  slots retain their item and count with an empty patch because no components
  were present on disk. Unknown schemas/types, invalid booleans, invalid legacy
  counts, truncation, and trailing bytes fail rather than being normalized.
- Production App/Sim/Storage capture uses the checked writer; storage owns
  the 2 MiB aggregate encoded entity-tail and 3 MiB whole-record limits and
  stages checked overlay application before replacing its target. A malformed
  entity rejects the whole load rather than creating an empty container.
- `encode_block_entity` remains an explicitly lossy writer for legacy callers.
  The experimental scheduler still uses legacy capture; switching that path
  and crash-atomic updates across player/overlay/journal records remain open.
- Sign lines and colors have matching encode/decode caps. Checked encoding
  rejects oversized UTF-8 fields without splitting characters or truncating
  content.

## Network encoding (`network` module)

- `PalettedContainer::encode_network` writes the 1.21.5+ wire layout:
  `bits_per_entry` byte, palette, then the packed long array with **no length
  prefix** (the client recomputes the long count from `bits_per_entry` and
  `CAPACITY`). Single: `bpe = 0`, one `VarInt` value, no longs. Indirect:
  `VarInt` palette length + ids, then `ceil(CAPACITY / (64 / bpe))` longs. Direct:
  no palette, longs repacked to the `direct_wire_bits` argument.
- In-memory direct storage is 32 bits per entry for lossless ids, but the wire
  uses the vanilla width (15 for block states), applied only at encode time. A
  flat world never reaches the direct representation.
- `encode_chunk_section_data` emits exactly `SECTION_COUNT` (24) sections
  bottom-to-top, each `i16` non-air count + block-states container + a
  single-valued `plains` (id 0) biomes container.
- `pack_motion_blocking_heightmap` returns 37 longs (256 columns, 9 bits each,
  7 per long). Each entry is `(highest_y + 1) - MIN_Y`, or `0` for an all-air
  column.
- `ChunkLightData::full_bright` marks all `LIGHT_SECTION_COUNT` (26) sections in
  the sky-light and empty-block-light masks (`0x03FF_FFFF`), supplies 26 `0xFF`
  2048-byte sky arrays, and leaves block light and the other two masks empty.
