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

## Explicit UUID entity mode (Format5)

- `PersistentEntityStore` is separate and dyn-compatible; unsupported defaults
  and Legacy/Metadata backends refuse even empty requests. UUID-mode instances
  refuse all four old wire-keyed entity methods before processing any input.
- Persistent keys preserve WorldId/DimensionId and all 128 UUID bits. Chunk
  origin plus local offset 15 must fit i32 block coordinates. Simulation bodies
  remain opaque and at most 256 KiB; envelope schema, payload schema, generator
  identity and database layout are independent versions.
- Format5 has exactly 12 typed tables. Entity24/FCUE22, locator20/12,
  chunk-index32/empty and epoch16/8 grammars are exact big-endian byte encodings.
  A world/UUID has at most one dimension. Relationships never repair themselves.
- Runtime calls check encountered local records, world locators, precise chunk
  indexes, positive epochs and real scope descriptors. They do not discover
  unknown orphan rows with a global runtime scan. Existing Format5 opens run
  the complete streaming validator once, checking both relation directions,
  all 12 grammars, descriptors, contiguous journal IDs/watermark, receipt shape,
  range existence and nonoverlap. No declaration, flat baseline or seed 0 is
  invented. Unscoped players/journal/receipts require at least one descriptor.
- Savebatch admits at most 64 proposals and 4 MiB of 24+22+payload bytes. Duplicate
  scoped keys and world/UUIDs refuse before mutation. Same UUID bits in distinct
  worlds are independent. Save changes chunks within a dimension; move requires
  same world/UUID, different dimensions, existing source and vacant destination.
- Exact expected identities, encountered relationships and all epoch increments
  preflight in the mutation boundary. Each distinct changed chunk increments
  once, at most 128 chunks per batch. Consistent raw no-op at u64MAX is allowed;
  a real change at MAX refuses the whole operation. Missing delete returns false.
  Historical empty-chunk epochs persist forever and prevent fresh rebinding.
- Memory holds all UUID records/indexes/epochs under one owned state lock within
  the shared mutation boundary; no guard crosses await. redb checks and writes
  within one real writer transaction on a blocking worker. Returned-error abort
  leaves all rows unchanged; Memory application has no fallible returned-error
  cutpoint after admission (allocation failure is not represented as rollback).
- Pages admit 1..64 entries and at most 1 MiB of 24+22+payload bytes. First range
  includes the raw Chunkprefix16, detecting short prefix keys; subsequent ranges
  exclude the last emitted UUID. Prefix upper bound never wraps. Reads inspect
  at most limit+1 indexes and one borrowed bounded lookahead before payload copy.
- Private transient cursors bind a Weak backend incarnation, full ChunkKey,
  complete expected identity, epoch and last emitted UUID. Arc clones share
  authority; foreign backend/reopen, chunk, identity and epoch each have a typed
  refusal before publishing entries. Cursors never extend backend lifetime.
- UUID-mode PluginStore Put/Get/Delete/Keys admit at most 4096 UTF-8 namespace
  bytes before key allocation/read/write. This is a Format5 policy; generic
  PluginId and Legacy/Metadata contracts retain their previous behavior.
- Main redb cache is 64 MiB; the disposable receipt range index adds 16 MiB cache.
  One decoded bounded row, bounded input/encoding plan/page, redb internals and
  object/map overhead are additional. Page/request byte limits are not a total
  process-heap guarantee. Memory retains the complete owned stored data.
- Receipt validation owns a uniquely created temporary file and removes only
  that exact regular-file identity after releasing its transaction/database.
  Replaced paths are preserved. Nonempty receipt ranges currently require Unix
  file identity support; unsupported platforms refuse explicitly.
- Existing formats3/4 refuse the UUID opener with migration-required and retain
  raw rows/catalog. Existing5 validation performs no normalization writes.
  No physical source-hash/recovery guarantee or power-loss proof is inferred.
  Legacy openers and 3→4 migration remain separate unchanged contracts.
- UUID save/delete/move are not ownership CAS, durable operation receipts,
  replay-safe general retries or Player↔Entity transactions. App/Sim activation,
  gameplay codecs, load reservations, transfer durability and 4→5 migration are
  separate work; this producer does not claim playable entities or item safety.
