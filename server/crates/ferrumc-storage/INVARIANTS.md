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
  format; format-4 opening of format 3 requires the separate explicit offline
  migration API, never an automatic upgrade.
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

## Explicit offline generation migration (WG-002-MIG)

- Only `prepare_generation_migration` can prepare format 3 -> 4; normal openers
  never upgrade. Legacy flat-v1 / 1.21.8 / Overworld / seed 0 is explicitly
  operator-declared. Seed 0 is assigned deliberately, never reconstructed.
- A declaration contains 1..=4096 unique typed scopes. Every scoped record is
  covered. Existing unscoped players, journal, receipts (even empty receipts)
  or watermark require exactly one declaration scope; ambiguity is refused.
- Full catalog/type/metadata/key/record validation streams one row at a time.
  Raw encoded records are never normalized by decoding/reencoding. Opaque
  entity/player payload validity beyond the current codec remains their owner.
  Migration plugin IDs are explicitly capped at 4096 UTF-8 bytes; existing
  plugin key/value caps remain 512 bytes/1 MiB.
- Journal key IDs begin at zero, are contiguous, match record IDs and exactly
  agree with the watermark. Every nonempty receipt range exists and is within
  MAX_SAVE_BATCH; distinct receipts cannot overlap. A private disposable disk
  index sorts ranges with 16 MiB cache, without materializing all receipts.
- The prepared source remains exclusively redb-locked until migration/drop.
  A separately create_new-created logical backup reproduces all existing
  raw key/value bytes, is closed, fsynced and completely verified. Its lock
  survives redb close/open and closed-file SHA-256 computation. Reuse requires
  the caller's recorded physical hash plus the exact logical snapshot.
- Prepare/refusal do not change FerrumC tables, records or format marker.
  redb open/close may change physical recovery/allocator bytes: unchanged
  physical source SHA-256 is deliberately not the contract. Dirty/recovery-
  required source files are refused before opening via a bounded header/
  geometry guard pinned to redb 2.6.3 and an aborting repair callback.
- Source/file path identity and backup hash are checked again immediately
  before commit. In the same real source writer transaction, the complete
  current snapshot is compared to the prepared incremental logical SHA-256,
  then all descriptors, new table and format marker 4 commit atomically.
- Format-4 retry is successful only after full data validation and exact
  equality of the complete descriptor set and caller declaration. Unknown,
  inconsistent, malformed or partial states are never repaired as migration.
- File identity/directory-fsync support currently requires Unix; unsupported
  platforms refuse explicitly. Source/backup redb caches are 64 MiB each;
  one record's decoded heap and redb internal overhead are additional.
- Rollback is explicit manual offline restoration of the verified format-3
  backup, after closing all handles and separately preserving the replaced
  state. No blind backup overwrite or automatic rollback after an ambiguous
  commit. Processkill/power-loss evidence and App/Sim activation remain separate.
