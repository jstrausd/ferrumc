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
- The production `RedbStore::open` keeps database format 3 and its exact
  eight-table catalog. Only `open_with_generation_metadata` supports format 4
  with a ninth `ferrumc:generation` table. Neither opener rewrites the other
  format; format-4 opening of format 3 requires explicit future migration.
  Chunk/overlay envelopes version 2 remain unchanged in both layouts.
  Inner block-entity payloads explicitly distinguish versioned lossless records
  from readable legacy sign/chest records; caller `SchemaVersion` is preserved.
- `InMemoryStore` is an owned struct, not a global. State is shared via `Arc`,
  not static mutable storage.
- Coordinates in keys are typed (`ChunkPos`), never raw tuples.
- Trait methods are `async` via `async_trait`, keeping the traits `Send + Sync`
  and dyn-compatible.

## Explicit generation-metadata mode (WG-002)

- Legacy instances and inherited metadata methods return `Unsupported`, never
  a successful absent descriptor. `InMemoryStore::new/default` remain legacy;
  `with_generation_metadata` enables the new contract explicitly.
- Every descriptor binds exactly one typed world/dimension key (eight
  big-endian bytes) to WG-001's exact 27-byte generation identity. Database
  format, descriptor envelope schema and generator revision are independent.
  At most 4096 descriptors exist; an existing exact compare still works at cap.
- Identical descriptors are idempotent; different seeds or rules are refused.
  There is no metadata update/delete API and no inferred legacy seed.
- Missing descriptors initialize only with globally empty chunks, overlays,
  entities, players, mutation journal, receipts and journal watermark. Empty
  overlays/empty receipts still count. Plugin KV alone does not count.
- The global freshness boundary is conservative because player and journal
  records lack scope handles. Scoped saves require their exact descriptor;
  player/journal writes require at least one descriptor, without claiming
  precise world attribution. Additional lifecycle/scoping work remains open.
- Save batches preflight all descriptor admissions before payload mutation;
  no failed batch changes earlier entries. Legacy admission is unchanged.
- redb performs compare/freshness/insert within one writer transaction and
  reports `Initialized` only after successful commit. All async DB operations
  remain in `spawn_blocking`; type/catalog mismatches are not auto-repaired.
- InMemory initialization acquires an owned exclusive mutation-boundary lock.
  Chunk/overlay/entity saves, their batches, both journal append paths, player
  saves and chunk/entity/player deletes acquire its shared guard before map
  locks. No such guard crosses an await. Plugin KV remains independent.
- Storage causes remain distinct: unsupported mode -> `Unsupported`;
  migration/missing/mismatch -> `InvalidState`; metadata cap -> `Capacity`;
  malformed key or WG-001 decode failure -> `Internal` with specific cause;
  wrong database version/catalog/table type -> existing `InvalidState` refusal.
  Operational I/O/lock/commit errors remain backend failures.
- WG-002 supplies neither format migration nor production App/Sim activation,
  natural terrain, configurable startup seeds, Vanilla-NBT interoperability,
  or process-kill/power-loss recovery evidence.
