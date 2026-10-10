# ferrumc-storage

Persistent storage traits with in-memory and durable redb backends.

This crate defines *what* the rest of the server can persist, behind async
traits, plus an owned [`InMemoryStore`] used by tests and a [`RedbStore`] for
durable transactional writes. redb operations run through `spawn_blocking`;
the application owns the bounded storage worker queue.

## Traits

- [`WorldStore`] — chunk columns ([`ChunkRecord`]) and entities
  ([`EntityRecord`]), addressed by typed [`ChunkKey`] / [`EntityKey`], plus
  edited-section overlays and the block-mutation journal.
- [`PlayerStore`] — per-player records ([`PlayerRecord`]) keyed by `PlayerId`.
- [`PluginStore`] — per-plugin private key-value storage. Every call is scoped
  to a `PluginId`, so a plugin can never read, enumerate, or delete another
  plugin's keys.

Reads of a missing key return `Ok(None)` (or an empty collection), never an
error.

## Records and versioning

Every record carries a [`SchemaVersion`], preserved verbatim across save/load,
so its owning layer can detect and refuse data written by an incompatible
build. No automatic pre-alpha rewrite is attempted. A `ChunkRecord` holds a
structured `Chunk`; `EntityRecord` and `PlayerRecord` carry a length-bounded
opaque payload owned by the simulation layer (a
`PlayerRecord` also keeps the typed `GameMode`).

Chunk and overlay envelopes retain format version 2, and the database header
retains format version 3. Checked writes use the versioned world block-entity
codec, including lossless item components and opaque component bytes. That
codec explicitly reads the previous unversioned sign/chest payloads; legacy
chest stacks become componentless stacks. The caller-owned `SchemaVersion`
does not select this inner grammar. Over-limit legacy records are rejected
without truncating or rewriting their persisted bytes.

## Checked chunk persistence

[`ChunkRecord::try_new`] and [`ChunkOverlayRecord::try_from_chunk`] reject an
invalid or oversized entity instead of omitting it. Overlay capture includes
every cumulatively edited section and every block entity, and leaves all dirty
masks intact on failure. [`ChunkOverlayRecord::try_apply_to_chunk`] validates
and stages the whole application, then replaces the target only on success.
Both stores validate complete saves before committing; a failing batch leaves
prior records unchanged. Chunk keys and values must name the same column, and
redb reads reject mismatched persisted columns.

The old infallible `new`, `from_chunk`, and `apply_to_chunk` entry points remain
for source compatibility. Legacy overlay capture can omit oversized entities
and discard item components, and legacy apply can skip malformed entities.
Durable consumers should use the checked entry points.

## Async strategy

The traits use [`async_trait`](https://docs.rs/async-trait). This boxes returned
futures as `Box<dyn Future + Send>`, which keeps the traits dyn-compatible (the
simulation holds `Arc<dyn WorldStore>`) and guarantees `Send` futures. Native
async-fn-in-trait was rejected: it cannot express the `Send` bound and trips the
`async_fn_in_trait` lint under `-D warnings`.

## Bounded shapes

Untrusted input is bounded before allocation: plugin keys
([`MAX_PLUGIN_KEY_LEN`]) and values ([`MAX_PLUGIN_VALUE_LEN`]), record payloads
([`MAX_ENTITY_DATA_LEN`], [`MAX_PLAYER_DATA_LEN`]), and batched saves
([`MAX_SAVE_BATCH`]). Opaque player/entity record decoding checks the remaining
payload before copying it. Chunk block-entity payloads share a 2 MiB encoded
tail allowance ([`MAX_CHUNK_BLOCK_ENTITY_DATA_LEN`]), including coordinates and
lengths; a whole full-chunk or overlay record is capped at 3 MiB
([`MAX_CHUNK_RECORD_LEN`]) before parsing. Per-entity world/item codec limits
also apply. These byte limits do not describe decoded heap usage.
[`ChunkOverlayRecord::encoded_len`] exposes the exact checked overlay byte size
without allocation so the worker can enforce byte-based backpressure.

## Not a global

[`InMemoryStore`] is a plain owned struct. Callers construct one and share it
(typically `Arc<InMemoryStore>`); there is no global mutable state.

## Invariants

See `INVARIANTS.md` in this directory.
