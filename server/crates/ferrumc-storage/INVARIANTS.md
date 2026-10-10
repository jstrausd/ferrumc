# Invariants: ferrumc-storage

> Rules that must hold for all code in this crate. Violating these is a bug.

## General

- No `unwrap()` or `expect()` outside `#[cfg(test)]`.
- No unbounded channels or allocations from untrusted input.
- All public items have rustdoc.
- Error types classify the failure mode.

## Crate-Specific

- Reads of a missing key return `Ok(None)` (or an empty collection), never an
  `Err`. An `Err` means the operation itself failed.
- Every record carries a `SchemaVersion`, preserved verbatim across save/load.
  Storage never reinterprets or rewrites it.
- `PluginStore` is namespaced per `PluginId`. A plugin can never read,
  enumerate, or delete another plugin's keys.
- Untrusted input (plugin keys/values, record payloads, save batches) is bounded
  before allocation and rejected with a classifying `StorageError`.
- Player/entity decoders check borrowed payload lengths before copying them.
  Full-chunk and overlay records are limited to 3 MiB before parsing; all block
  entities share a 2 MiB encoded tail allowance including coordinates/lengths,
  in addition to per-entity world/item codec limits. Encoded-byte caps are not
  decoded-heap caps. Existing over-limit records are refused without rewrites.
- Checked chunk capture preserves every current item component representation,
  including opaque bytes, or fails the whole capture. It never silently omits
  an entity or clears source dirty/edited masks. Legacy infallible overlay APIs
  retain their previous lossy behavior for source compatibility only.
- Checked overlay apply validates and stages all edits before replacing the
  target. Failed apply leaves blocks, entities, and dirty/edited masks intact.
- Stored chunk keys and values identify the same column. Entity positions are
  unique, belong to that column, and lie inside the fixed world height. Chunk
  origins and their maximum local offsets must fit the block coordinate domain.
- Both backends reject a failing save batch without changing earlier records.
- Database format 3 and chunk/overlay envelopes version 2 remain unchanged.
  Inner block-entity payloads explicitly distinguish versioned lossless records
  from readable legacy sign/chest records; caller `SchemaVersion` is preserved.
- `InMemoryStore` is an owned struct, not a global. State is shared via `Arc`,
  not static mutable storage.
- Coordinates in keys are typed (`ChunkPos`), never raw tuples.
- Trait methods are `async` via `async_trait`, keeping the traits `Send + Sync`
  and dyn-compatible.
