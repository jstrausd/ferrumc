# ferrumc-world

Pure world model. No threads, no DB, no packets.

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
  executable identity is Minecraft 1.21.8 / `FerrumC` flat revision 1 / Overworld.

## Generator contract

`FlatWorldGenerator::new()` and `default()` select numeric seed 0 and preserve
the existing fixed layers. `with_seed(WorldSeed::new(value))` retains another
signed 64-bit seed in its identity. Flat blocks remain seed-independent; these
seeds still identify different baselines. Both the existing `generate` method
and object-safe `ChunkGenerator::generate_chunk` use the same production path.

`GenerationIdentity::encode` returns a fixed 27-byte `FCGI` schema-1 envelope.
Its big-endian fields are magic, `u16` envelope schema, three `u16` Minecraft
release components, `u16` algorithm, `u32` generator revision, `i64` seed and
`u8` dimension. Algorithm 1 names `FerrumC` flat; dimension 0 names Overworld.
Envelope schema and generator revision are independent. `decode` requires the
exact length, allocates nothing, and classifies unsupported metadata without
falling back. `ensure_compatible` requires exact supported identity equality.
This envelope is `FerrumC` metadata, not Vanilla NBT or protocol data.

The storage/load path does not yet persist or compare this identity. A separate
storage contract and Sim/App adoption must guard regeneration and overlay
application before different/configurable terrain is activated. The existing
server still serves fixed-layer flat terrain. Natural Overworld generation,
Nether/End, textual/random seed selection and restart migration remain later
tickets. See [WG-001 notes](../../../docs/agent-tasks/WG-001_GENERATION_CONTRACT.md)
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
identity. Density routing/sampling, Normal-/Blended-/Simplex noise,
`WorldgenRandom` feature/decoration seeding, Overworld/Nether/End consumers and
world seed parity remain separate work.

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

## Invariants

See `INVARIANTS.md` in this directory.
