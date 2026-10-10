# Invariants: ferrumc-registry

> Rules that must hold for all code in this crate. Violating these is a bug.

## General

- No `unwrap()` or `expect()` outside `#[cfg(test)]`.
- No unbounded channels or allocations from untrusted input.
- All public items have rustdoc.
- Error types classify the failure mode.

## Crate-Specific

- The runtime is dependency-free and does no I/O or parsing. Block/dimension
  constants are hardcoded as `const`; item/block/entity-type/worldgen/biome data is generated at
  **build time** by `build.rs` from the vendored snapshots into static arrays emitted
  to `OUT_DIR`. Either way the runtime never parses JSON and pays zero boot cost
  (no `OnceCell`/`lazy_static`). JSON/TOML parsing exists only in `build.rs` and
  under `#[cfg(test)]`.
- `build.rs` MUST assert the item data's structural invariants (contiguous ids
  `0..=N`, every item carries a `u8`-sized `max_stack_size`) and panic loudly on
  drift, so a botched re-vendor fails the build instead of shipping wrong tables.
- Block-state ids are fixed protocol constants and MUST equal the default state
  of the corresponding block in the vendored `blocks.json` at the pinned commit.
  The drift-guard tests enforce this; do not change a constant without re-pinning
  and re-vendoring the fixtures + manifest checksums together.
- Biome and dimension-type wire ids are server-assigned dynamic-registry indices,
  not protocol constants. Biome resource identities may use opaque private local
  slots for checked resource lookup; never expose those slots as wire/persisted
  IDs or claim that canonical-name order is original bootstrap/network order.
- `manifest.toml` checksums and byte counts must match the vendored fixtures.
- Item placement bases retain the reverse-mapping snapshot except for the three
  explicitly declared standing/wall torch families. Their generated bases are
  the standing defaults resolved by name from `blocks.json`; the source mapping
  must still name the matching standing or wall default or the build fails.
  Source snapshots remain unchanged. Contextual attachment and support checks
  belong to placement/simulation, not registry lookup.
- This crate may depend only on `ferrumc-core`; never on sim, net, or storage.
- Entity-type tables derive exclusively from the unchanged CAT-02.1 original
  `docs/fixtures/minecraft-1.21.8-original/entity-types.json`. Build admission
  rejects inputs above 64 KiB, malformed/duplicate JSON fields, invalid names,
  duplicate/gapped/out-of-range ids, wrong registry/default/count and absent
  player. Its 151 static type identities do not implement entity metadata or AI.
- `EntityTypeId` is opaque and constructed only through checked name/id lookup;
  it has no unchecked/default/serialization path. Runtime lookups and ordered
  iteration allocate nothing, and unknown names/ids return `None` rather than
  defaulting to pig. Player identity is generated from the same source name.
- The Registry oracle helper verifies independent fixture/report/manifest pins
  using explicit requirements that remain active under Python `-O`. A write
  requires the complete SHA-pinned original report and equality of every
  entity-type name/id to the unchanged subset. Oracle identity comparison is
  separate from live Paper/gameplay differential acceptance.
- Built-in connection facts are pinned to Minecraft 1.21.8 / Paper Build 60.
  Fence families follow explicit `FENCES`/`WOODEN_FENCES` membership (including
  crimson and warped as wooden); exceptions follow the reviewed block classes,
  explicit names and `SHULKER_BOXES` membership. Tag facts retain checksummed
  source fixtures and are cross-checked against the vendored block registry.
- Unknown/malformed block names never become known non-exceptions. The facts
  do not include dynamic datapack tag overrides or classify full supporting
  faces: the registry's `boundingBox` flag is not support-shape evidence.
- Worldgen resources derive only from the unchanged CAT-02.1 `worldgen.json`.
  Build-only SHA256/byte pins admit that fixture, manifest and version manifest;
  their original resource receipts and 1.21.8/772/4440 pins must agree. The 60
  noise, 35 density and seven settings resources are complete for this builtin
  scope. Other worldgen families and dynamic datapacks need separate producers.
- JSON admission refuses duplicate decoded keys, wrong families/resource counts,
  malformed/foreign names, unknown fields/forms, invalid codec values and cycles.
  Whole worldgen input: 4 MiB/131072 nodes/depth32/container1024/string256 bytes.
  Each selected definition: 128 KiB/4096 nodes/depth32/container256/string256 bytes.
  Nodes include keys and root; depth counts keys and children from root zero.
  Bounds are enforced while parsing, before child containers are materialized.
- Noise parameters have immutable exact rounded f64 bits, emitted using
  `f64::from_bits`; original signed/zero amplitude positions remain intact.
  Builtin bounds are firstOctave -16..-1, 1..16 amplitudes, finite values 0..2
  and at least one nonzero amplitude. These are dataset bounds, not general
  Minecraft codec or datapack limits.
- Resource IDs are family-separated private local slots, with no raw/default/
  serialization constructor. Lookups accept only exact canonical or bare names;
  unknown stays `None`. Slots are not world, wire or persisted registry IDs.
- `OriginalDefinition` preserves the exact vendored export slice, including
  number tokens; it is not the differently formatted JAR resource byte string.
  Its immutable private fields expose measured stats and sorted unique typed
  dependencies. Density/Noise references resolve fully; external block/biome/
  codec types and synthetic RNG names remain separate categories. There are no
  builtin tag references; unsupported tags fail admission. Dependency charging
  precedes insertion: maximum 2048 occurrences and 512 distinct per definition.
- Reference paths follow the original codecs: Clamp.input is Directcodec,
  Shift*.argument is NoiseHolder, other unary.argument is DensityHolder. Missing
  references never become zero noise. Block properties are checked against the
  existing block producer's domains; external categories do not grant runtime
  block/biome/surface implementation.
- Definitions supply data only. World must independently check cumulative graph,
  work, seed and sampler bounds before compiling/evaluating. No runtime JSON
  parser, sampler, fallback seed, density compiler or chunk generation is added.
- All 65 biome and two multinoise preset definitions preserve exact RawValue
  slices from the unchanged original worldgen export. Fixed original export,
  manifest and version pins plus the 65+2 original resource receipts establish
  authority. Whole/definition syntax limits stay identical to the Worldgen
  producer. Full Biome.DIRECT data is retained, without new runtime biome codec
  execution or reduced NETWORK codec payload claims.
- Expanded preset data derives from the unchanged independently frozen genuine
  original-method census: 7593 Overworld and five Nether rows. The production
  JSON and provenance have independent fixed byte/SHA pins. Original normal/-O
  origin receipts, helper, historical runner and Root freeze remain unchanged.
  No copied biome formulas or alphabetic Prismarine numeric ID authority is used.
- Census parsing is build-only, typed and field-bounded: 2 MiB/262144 nodes/
  depth8/string64 bytes; root has exactly two presets, preset arrays exactly
  7593/5 rows, rows exactly four fields, axes exactly six two-integer spans.
  Charges precede child materialization; an extra array element fails before
  decoding its payload. Duplicate decoded object keys, unknown fields/references,
  noncontiguous indexes, floating values and integer overflow are refused.
- Quantized spans obey -20000 <= minimum <= maximum <= 20000 and offset
  0..=10000, matching the original codec domain. All original long values,
  parameter order and repeated entries remain intact; row ordinal is not an ID.
  The immutable 7598-entry static table occupies at most 850976 native bytes;
  lookups, definitions, borrowed slices and iterators allocate nothing.
- The dependency-free runtime provides opaque family-separated biome/preset
  resource IDs, never unchecked/default/serialization constructors. Selection,
  RTree history, dynamic configuration order, wire NBT, chunk palette mappings,
  active generation and gameplay require separate consumers. Existing flat
  plains string/configuration consumers and Worldgen biome dependencies are
  unchanged; this producer supplies no runtime registration or fallback biome.
