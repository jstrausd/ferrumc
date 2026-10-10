# Invariants: ferrumc-world

> Rules that must hold for all code in this crate. Violating these is a bug.

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
  by these primitives. The separate seed-free compiler exposes metadata only.
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
  density adapter must cast original f64 coordinates to f32 before sampling
  and widen the f32 result after sampling, as the original does.
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
  No raw JSON compiler, unchecked node handle, default router, sample method,
  seeded noise, natural-world identity or chunk output is exposed.
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
