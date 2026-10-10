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

Supported reads of a missing key return `Ok(None)` (or an empty collection).
An unsupported metadata contract returns `Unsupported`, independently of
whether a record exists.

## Records and versioning

Every record carries a [`SchemaVersion`], preserved verbatim across save/load,
so its owning layer can detect and refuse data written by an incompatible
build. No automatic pre-alpha rewrite is attempted. A `ChunkRecord` holds a
structured `Chunk`; `EntityRecord` and `PlayerRecord` carry a length-bounded
opaque payload owned by the simulation layer (a
`PlayerRecord` also keeps the typed `GameMode`).

Chunk and overlay envelopes retain format version 2. The production
[`RedbStore::open`] header and eight-table catalog retain database format 3;
explicit generation-metadata mode uses format 4. Checked writes use the versioned world block-entity
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

## Opt-in immutable generation metadata

[`RedbStore::open_with_generation_metadata`] creates/opens database format 4,
with a ninth `ferrumc:generation` table. Keys are exactly eight big-endian
world/dimension bytes; values are the fixed 27-byte WG-001 generation identity.
The descriptor schema and generator revision are independent of database format.
[`InMemoryStore::with_generation_metadata`] provides the matching in-memory
contract. Existing constructors retain production behavior and return
`Unsupported` for metadata methods, as do the trait defaults.

```rust
use ferrumc_core::{DimensionId, WorldId};
use ferrumc_storage::{GenerationKey, InMemoryStore, WorldStore};
use ferrumc_world::{GenerationIdentity, WorldSeed};

# async fn example() -> ferrumc_core::Result<()> {
let store = InMemoryStore::with_generation_metadata();
let scope = GenerationKey::new(WorldId::new(0), DimensionId::new(0));
let identity = GenerationIdentity::flat_overworld(WorldSeed::new(-42));
store.ensure_generation_identity(scope, identity).await?;
assert_eq!(store.load_generation_identity(scope).await?, Some(identity));
# Ok(())
# }
```

Confirmation is immutable and idempotent: the first committed insert returns
[`GenerationInitialization::Initialized`], and exact retries return
[`GenerationInitialization::AlreadyPresent`]. Mismatch preserves the stored
identity. There are no update/delete operations. The descriptor count is
bounded by [`MAX_GENERATION_IDENTITIES`] (4096); existing compares work at cap.

A missing descriptor can initialize only while the **entire store** has no
world state: chunks, overlays (including empty overlays), entities, players,
journal rows, journal receipts (including empty batches), and journal watermark.
Plugin KV alone permits initialization. Existing state never proves a historical
seed or allows an automatic binding to the flat generator's numeric default 0.

In metadata mode, chunk/overlay/entity saves require a descriptor for their
exact scope. Player and journal saves, including empty journal requests,
require at least one confirmed store descriptor. These records currently lack
scope handles, so this last condition does not identify their precise world.
Multiple scopes can be initialized before world state exists; later namespace
lifecycle and scoped records need separate work. All batch admissions are
checked before payload mutation, and rejected requests leave state unchanged.
redb uses one writer transaction for admission and write/commit; [`InMemoryStore`] uses
an owned exclusive initialization boundary shared by saves and deletes.

Format-4 opening of format 3 returns the classified
[`StorageError::GenerationMigrationRequired`] and changes no logical records,
header or catalog. The old opener refuses format 4. Wrong table shapes/catalogs
use the existing `InvalidState` format refusal; malformed generation keys and
WG-001 values map to `Internal` with their specific diagnostic cause. Missing
or different descriptors map to `InvalidState`, and the count limit maps to
`Capacity`. Operational backend errors remain internal backend failures.

This opt-in API is not wired into App/Sim. Consumer activation remains a
follow-up ticket; no natural world generation or
startup seed selection is activated here. Transaction abort/drop and lost
response tests cover atomicity and retry behavior, not process-kill/power-loss
durability or runtime parity with Paper.

## Explicit offline format migration

[`RedbStore::prepare_generation_migration`] completely validates an existing
format-3 store while holding an exclusive source lock. The operator must
explicitly assert [`LegacyGenerationDeclaration::ferrumc_flat_v1_seed_zero`]:
1.21.8 / `FerrumC` flat-v1 / Overworld with deliberately assigned seed 0, and a
complete set of 1..=4096 unique typed scopes. No seed is guessed from chunks or
configuration. Unscoped players/journal/receipts/watermark require exactly one
declared scope. Wrong catalog/type/metadata, malformed or over-limit records,
missing scopes, discontinuous journals and overlapping receipts are refused.

```no_run
use ferrumc_core::{DimensionId, WorldId};
use ferrumc_storage::{
    GenerationKey, GenerationMigrationError, GenerationMigrationPreparation,
    LegacyGenerationDeclaration, RedbStore,
};

# fn offline() -> Result<(), GenerationMigrationError> {
let scope = GenerationKey::new(WorldId::new(0), DimensionId::new(0));
let declaration = LegacyGenerationDeclaration::ferrumc_flat_v1_seed_zero([scope])?;
match RedbStore::prepare_generation_migration("world.redb", declaration)? {
    GenerationMigrationPreparation::Required(prepared) => {
        // Present/store this report before the operator initiates backup/commit.
        let report = prepared.report().clone();
        let backed_up = prepared.create_backup("world-before-format4.redb")?;
        let backup = backed_up.backup().clone();
        // Retain report and backup evidence even if the following result is lost.
        let outcome = backed_up.migrate()?;
        assert_eq!(outcome.report(), &report);
        assert_eq!(outcome.backup(), &backup);
    }
    GenerationMigrationPreparation::AlreadyCompatible(report) => {
        assert_eq!(report.source_format(), 4);
    }
}
# Ok(())
# }
```

Only [`BackedUpGenerationMigration`] exposes `migrate`: a separately
`create_new`-created, closed/fsynced and fully verified backup is mandatory.
Backup copies original raw rows without record normalization. The report's
incremental logical SHA-256 differs from the backup's closed physical-file
SHA-256; redb can alter recovery/allocator housekeeping bytes during open/close.
Existing files are never overwritten. An explicitly reused backup requires
its previously recorded file hash and exactly matching validated logical rows.
The source/backup locks survive report, verification and commit; the complete
source snapshot, path identity and backup hash are rechecked before the atomic
generation-table/descriptors/format-4 commit. A format-4 retry validates all
records and the complete exact declaration, rather than merely its marker.

These methods perform synchronous offline I/O/CPU work and require Unix inode
identity/directory fsync in this release. Source/backup caches are bounded to
64 MiB each; receipt validation uses a disposable disk index with 16 MiB cache,
and existing record-codec caps still apply. This is no whole-world heap copy or
guaranteed total RSS bound. Recovery-required files need separate recovery;
there is no automatic startup upgrade or data repair. Failed backup attempts
may leave their own incomplete file for operator inspection.

Rollback requires closing every handle, separately preserving the state being
replaced, and explicitly restoring the verified format-3 backup offline. No
automatic overwrite follows an ambiguous commit, and the backup excludes later
gameplay changes. Operator CLI, runtime metadata activation, natural generation
and processkill/power-loss acceptance remain separate tasks.

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
