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
`encode_block_entity` retains that legacy, lossy writer for existing storage
callers. Adopting the new fallible codec in overlay capture and making overlay
application atomic remain separate `ferrumc-storage` work; this world change
alone does not activate chest component persistence in the running server.

## Invariants

See `INVARIANTS.md` in this directory.
