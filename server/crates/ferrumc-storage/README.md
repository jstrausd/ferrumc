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

## Explicit UUID entities: Format5

[`PersistentEntityStore`] persists full [`ferrumc_core::EntityUuid`] values,
independently of runtime wire IDs. Use [`InMemoryStore::with_uuid_entities`] or
[`RedbStore::open_with_uuid_entities`] explicitly. Legacy and metadata-only
instances return `Unsupported`; UUID mode refuses all old wire-keyed entity
methods, including empty batches. The application does not activate this mode.

```rust
use ferrumc_core::{DimensionId, EntityUuid, WorldId};
use ferrumc_math::ChunkPos;
use ferrumc_storage::{EntityRecord, GenerationKey, InMemoryStore,
    PersistentEntityKey, PersistentEntityRecord, PersistentEntityStore,
    PersistentEntityWrite, SchemaVersion, WorldStore};
use ferrumc_world::{GenerationIdentity, WorldSeed};

# async fn uuid_example() -> ferrumc_core::Result<()> {
let store = InMemoryStore::with_uuid_entities();
let world = WorldId::new(0);
let dimension = DimensionId::new(0);
let identity = GenerationIdentity::flat_overworld(WorldSeed::new(42));
store.ensure_generation_identity(GenerationKey::new(world, dimension), identity).await?;
let key = PersistentEntityKey::new(world, dimension, EntityUuid::from_bytes([1; 16]));
let record = PersistentEntityRecord::new(ChunkPos::new(0, 0),
    EntityRecord::new(SchemaVersion::new(7), vec![1, 2, 3])?)?;
store.save_persistent_entity(PersistentEntityWrite::new(key, identity, record)).await?;
assert!(store.load_persistent_entity(key, identity).await?.is_some());
# Ok(())
# }
```

Saves admit at most 64 distinct world/UUID proposals and 4 MiB encoded bytes.
One entity occupies24 key bytes plus22 envelope bytes plus its at-most 256 KiB
opaque body. Exact expected generation identities and encountered relations
are checked atomically. Save can change chunks in the same dimension; the
separate move operation changes dimensions for the same world/UUID. Each
changed chunk advances its positive persisted epoch once per commit; historical
empty chunks retain epochs. An identical, consistent raw save is a no-op,
including at the epoch maximum. Real changes at the maximum refuse atomically.

Pages return 1..64 entries and at most 1 MiB of the same encoded-byte accounting,
with at most one borrowed lookahead. Private transient cursors bind live backend,
chunk, complete identity and epoch. Arc clones share authority; reopen creates a
new incarnation. [`PersistentEntityCursorRejection`] classifies owner, chunk,
identity and epoch mismatches. Cursors retain no database lifetime. A page with
no continuation proves encountered chunk-prefix exhaustion; point calls do not
search the world for unknown orphans.

Format5 retains the nine metadata-layout tables and adds entity locator, chunk
index and epoch tables. Existing Format5 opens validate the complete 12-table
catalog, types, row grammars, both entity-index directions, actual descriptors
and unchanged journal/receipt rules once. Validation streams one bounded record
at a time and uses a disposable disk receipt index with 16 MiB cache; the main
redb cache is 64 MiB. Decoder heap, bounded request/plan/page buffers, redb internals
and object/map overhead are additional to caches and encoded-byte limits.
Receipt-index cleanup checks owned file identity. Validation of nonempty receipt
ranges currently requires Unix and refuses unsupported platforms explicitly.

UUID mode bounds `PluginStore` namespace IDs to 4096 UTF-8 bytes on all four
methods before key allocation/read/write. This Format5 policy makes admitted
plugin rows valid at strict reopen; generic `PluginId` and older mode contracts
remain unchanged. Plugin keys and values keep their existing bounds.

Existing formats3/4 receive [`StorageError::PersistentEntityMigrationRequired`]
without raw-row/catalog changes. Format4→5 requires the explicit offline API
below; this opener performs no migration or normalization. Existing formats5
are not claimed physically hash-invariant across redb open/recovery. Save,
delete and move provide no ownership CAS, general replay-safe retry, durable
operation receipt or Player↔Entity transaction. Gameplay codecs, entity-load
reservation, App/Sim activation and transfer durability remain separate.

## Explicit offline Format4 → Format5 migration

[`RedbStore::prepare_uuid_entity_migration`] validates the exact nine-table
Format4 catalog and every bounded record using actual stored generation
identities. Multiple worlds, dimensions and seeds are retained. Every old
wire-keyed entity row refuses migration, including a valid empty body; storage
does not invent persistent UUIDs or chunk membership. Format3 must first use
the separate generation migration. The older openers and 3→4 API are unchanged.

```no_run
use ferrumc_storage::{RedbStore, UuidEntityMigrationError, UuidEntityMigrationLimits};

# fn offline() -> Result<(), UuidEntityMigrationError> {
let limits = UuidEntityMigrationLimits::new(64 * 1024 * 1024, 16 * 1024 * 1024)?;
let prepared = RedbStore::prepare_uuid_entity_migration("world.redb", limits)?;
let backed = prepared.create_backup("world-before-format5.redb")?;
// A durable, immutable checkpoint is required before source commit is possible.
let ready = backed.write_checkpoint("world-format5.checkpoint")?;
let outcome = ready.migrate()?;
assert_eq!(outcome.report().source_format(), 4);
assert_eq!(outcome.report().target_format(), 5);
# Ok(())
# }
```

Only [`CheckpointedUuidEntityMigration`] exposes `migrate`. The backup copies
all nine raw tables, closes, fsyncs and records its physical SHA256. Every
verification opens a fresh bounded physical copy, never the recorded original
backup. Reuse without a checkpoint requires the previously recorded closed
backup hash. Source, backup and checkpoint roles must be separate regular
files with checked Unix device/inode identities; locks fail immediately with
`Busy`. Final symlinks and unexpected hardlink aliases refuse.

The fixed 344-byte checkpoint binds source inode, immutable backup inode/length/
hash, twelve logical counters, the Format4 snapshot and projected Format5
snapshot. Publication writes/fsyncs a uniquely named staging file, links the
final path without overwrite, fsyncs its directory and removes only the known
staging alias. Failed publication never grants commit authority. Checked
resume can stabilize the exact known final/staging pair after a lost reply.

`migrate` revalidates the complete source, backup and checkpoint. Its one real
`Durability::Immediate` transaction changes only the marker 4→5 and creates
three empty UUID-index tables. Every common raw row is compared to the backup;
the migration fingerprint includes table presence, types and raw row lengths.
`CommitUncertain` requires checked resume, because an I/O error is not proof
that the source transaction aborted. The checkpoint never gains a commit flag.

```no_run
use ferrumc_storage::{RedbStore, UuidEntityMigrationError,
    UuidEntityMigrationLimits, UuidEntityMigrationResume};

# fn offline_resume() -> Result<(), UuidEntityMigrationError> {
let limits = UuidEntityMigrationLimits::new(64 * 1024 * 1024, 16 * 1024 * 1024)?;
let outcome = match RedbStore::resume_uuid_entity_migration(
    "world.redb", "world-format5.checkpoint", "world-before-format5.redb", limits,
)? {
    UuidEntityMigrationResume::Ready(ready) => ready.migrate()?,
    UuidEntityMigrationResume::AlreadyCompleted(outcome) => outcome,
};
assert_eq!(outcome.report().target_format(), 5);
# Ok(())
# }
```

Resume proves the exact captured Format4 state or its exact Format5 delta. A
valid Format5 store changed by gameplay, including entity save/delete with
retained epochs, refuses the old migration retry. Source physical length/hash
are informational; redb housekeeping is allowed. The original backup's physical
hash remains binding. Resume never restores a backup or rewrites evidence.

Both scratch budgets must be positive and their sum plus the 344-byte staging
extent must fit u64. The verification copy and receipt-index backends refuse
`set_len` and checked write extents above their limits before delegation. At
most one of each exists per operation. Main/source/backup verification caches
are each 64 MiB, with at most two active simultaneously; the receipt index adds
16 MiB. A 64 KiB copy/hash buffer and one bounded decoded row are additional,
as are redb/allocator/OS overhead. These are file-extent/cache bounds, not a
total RSS or host-disk quota; the permanent backup and source growth are extra.
Cleanup checks owned identity and preserves foreign replacements, reporting
failure explicitly. An unverifiable newly created file identity is also a
classified cleanup failure, rather than permission to delete its path.

These methods perform synchronous offline I/O and currently require Unix.
There is no operator CLI, automatic startup migration, App/Sim activation,
Anvil import or entity codec in this ticket. Tests of actual producer abort,
commit-I/O and lost replies do not establish process-kill/power-loss durability
or Paper gameplay parity.

## Opt-in atomic Player/UUID-entity ownership (Format6)

[`PlayerEntityOwnershipStore`] is a separate dyn-compatible authority. Enable it
explicitly with [`InMemoryStore::with_player_entity_ownership`] or
[`RedbStore::open_with_player_entity_ownership`] and an [`OwnershipConfig`].
Formats3/4/5 and trait defaults refuse all seven ownership calls. Existing
formats require a separately specified offline migration; this producer never
upgrades them during open. Format6 has exactly15 typed tables: the Format5
catalog plus player ownership, entity ownership and ownership receipts.

[`ScopedPlayerKey`] binds the full global player UUID to an explicit generation
scope. Entity history remains keyed by world/full UUID. Each resource has a
private Fresh/Live/Tombstone stamp and a positive, nonwrapping revision once
changed. Deletion retains its header permanently. A different player scope or
entity dimension refuses rather than selecting another descriptor. Full
identities, payload schema versions and typed `GameMode` are preserved.

A coherent [`OwnedPlayerEntitySnapshot`] captures both records and stamps in
one actual read boundary. [`PlayerEntityTransferRequest`] changes exactly one
player and one entity in that same scope. Receipt-backed standalone player and
entity CAS calls support bootstrap/autosave without inventing a placeholder.
The old blind [`PlayerStore`], wire-entity and UUID-entity writers, including move,
refuse in Ownership mode. Read-only UUID point/page calls use the same owner
state and validate page lookahead before publishing entries.

```rust
use ferrumc_storage::{PlayerEntityOwnershipStore, PlayerEntityTransferRequest,
    PlayerOwnershipProposal, EntityOwnershipProposal, OwnershipTransferId,
    ScopedPlayerKey, PersistentEntityKey, PlayerRecord, PersistentEntityRecord,
    OwnershipResult};
use ferrumc_world::GenerationIdentity;

# async fn submit(store: &dyn PlayerEntityOwnershipStore,
#     player: ScopedPlayerKey, entity: PersistentEntityKey,
#     generation: GenerationIdentity, operation: OwnershipTransferId,
#     player_record: PlayerRecord, entity_record: PersistentEntityRecord,
# ) -> OwnershipResult<()> {
let before = store.load_owned_pair(player, entity, generation).await?;
let request = PlayerEntityTransferRequest::new(
    operation,
    before.player().expectation().clone(),
    before.entity().expectation().clone(),
    PlayerOwnershipProposal::put(player_record),
    EntityOwnershipProposal::put(entity_record),
)?;
let query = request.receipt_query();
let outcome = store.commit_player_entity_transfer(request).await?;
assert_eq!(store.load_ownership_receipt(query).await?.as_ref(),
    Some(outcome.receipt()));
# Ok(())
# }
```

The caller supplies a128-bit operation ID. A canonical SHA256 binds kind, scope,
generation, resource IDs, expected stamps and complete proposals, including
schemas/GameMode/chunk/opaque bytes. Constructors compute it synchronously
within hard admission; async workers use the cached digest. A matching retained
receipt wins before live-owner/CAS/history-capacity checks, allowing the exact
committed retry after later writes, deletion, reopen or a lost reply. Reusing an
ID for another request returns `ReceiptCollision`. Receipt results describe that
historical operation and do not assert that its stamps are still current.

Memory preflights and publishes player/entity payloads, UUID relationships,
epochs, headers, receipt and counters under one owned state lock. redb preflights
and writes all of them in one `Durability::Immediate` writer transaction on a
blocking worker. Real changes increment each resource once and each affected
chunk once; raw-equal no-ops leave revisions/epochs unchanged but consume a new
receipt. Fresh/Tombstone deletion returns `AlreadyAbsent`. MAX revisions and
epochs permit only consistent raw no-ops and never wrap.

[`OwnershipCapacity`] fixes canonical-request, receipt-row and revision-row
limits. Request admission is149..2MiB; current maximum player1MiB plus entity256KiB
forms1,310,896 canonical bytes. A receipt is256 value bytes/272 key+value bytes;
each retained resource header is55 key+value bytes. Capacity accounting is
checked. Limits are persisted exactly and must agree on reopen; history is not
pruned or reset. [`OwnershipOpenLimits`] bounds full startup scan rows/bytes,
actual journal-index scratch extent and1..64 concurrent ownership workers.
Arc clones share fail-fast `Busy` admission without an awaiting queue. A dropped
caller does not release a running blocking worker's permit. These bounds do not
cover caller-owned payloads/futures, all database caches, allocator overhead or
total process RSS. Memory retains its complete stored state.

Format6 startup validates every table/type/meta key, bounded record and both
UUID relationship directions, owner last-changing receipts, historical stamps,
epochs, counts, descriptors and the unchanged journal grammar. It streams a
bounded row before decoding/copying and does no normalization. The new validator
reuses typed raw/codec/engine parsers directly; malformed records retain their
specific class instead of being reconstructed from a `ServerError` string. The
Format5 validator and migrations remain unchanged. Main redb cache is64MiB; a
nonempty journal-range index adds16MiB cache within its separate scratch extent.
Scratch cleanup removes only its owned file identity and preserves replacements.

Precommit returned errors abort all actual rows. An actual commit I/O error is
`CommitUncertain`; a started worker panic or lost reply is `ExecutionUncertain`,
both carrying the original receipt query. Keep pending ownership until a checked
receipt or exact retry clarifies it. `None` from a receipt read is a snapshot,
not proof that another running worker aborted. Gameplay consumers must reserve
and publish inventory/entity changes at their own simulation boundary after
confirmation; storage does not interpret item quantities or plugin mutations.

Tests exercise independent fixtures/state traces, actual concurrent pickups and
autosave conflicts, every real row/commit cutpoint, actual commit-sync failure,
lost replies, cancelled callers, MAX histories, full scans/scratch boundaries
and maximum opaque payloads across both backends and reopen. Paper1.21.8 Build60
source/event ordering was inspected; no Paper runtime, client/plugin differential
acceptance or process-kill/power-loss evidence is claimed. App/Sim activation,
owned gameplay codecs, drop/pickup consumers, capacity changes and Format5→6
migration remain separate tickets.

The explicit synchronous Unix-only Format5→6 ownership migration additionally
requires caller-selected source, mapping and scratch paths. Preparation fully
validates source records and mapping, then `prepare_files` creates new Backup,
Plan, Stage and Checkpoint evidence. Actual private Singles CAS generates the
Stage histories before one atomic Source import. Source Player/Entity payloads
remain opaque and byte-identical; historical entity epochs advance by their
actual per-chunk entity counts. Resume requires all six paths and the exact
capacity/limit agreement, and distinguishes original5 from exact committed6.
This API returns offline evidence/outcomes only; it does not enable gameplay or
migrate player gameplay codecs. See the DUR-03.2 brief for exact mapping/plan/
checkpoint grammars and logical/physical resource bounds.

## Opt-in joint Player/Block durability (Format7)

[`PlayerBlockDurabilityStore`] binds one Live Player and 1..=8 complete Chunk
full/overlay/block-entity images to one atomic operation. Enable it explicitly
with `InMemoryStore::with_player_block_durability` or
`RedbStore::open_with_player_block_durability(path, config, scratch_directory)`;
every other mode refuses its five methods with `Unsupported`, and existing
format3..6 files return `MigrationRequired` (the offline Format6→7 migration is a
separate ticket). Callers load an `OwnedBlockSnapshot`, derive complete
postimages plus guards and ordered `ScopedBlockEffect`s from it, and submit a
`PlayerBlockRequest`, `BlockMutationRequest` or `ChunkImageCasRequest`. The
backend replays the effects on the resolved original and compares every cell
and block entity with the postimage; commits are Immediate, replay is byte-exact
by operation ID, and uncertain outcomes are resolved only through
`load_block_receipt` or an exact retry. Format7 refuses all blind chunk/overlay/
journal writers. Reads prove each touched column's own history entry (whole
receipts are re-derived by lookup, replay and reopen), so any 1..=8-column union
fits the hard cell budget whatever its history. A receipt shared by a call's
columns is read and digested once per call, and block entities are charged their
exact canonical encoding length (16 KiB is only their form bound). Unions of a few
distinct maximal receipts (from three effect-dense ones) can still exhaust the
512 MiB byte cap, which refuses typed. The redb opener checks its
scratch directory before touching the database path; a refused fresh create
leaves no file. See `INVARIANTS.md` for the complete contract and its limits.

Format7's scoped Journal startup pass holds one borrowed redb receipt row and its
parsed Q for the current contiguous operation range. It reads/hashes/parses that
receipt once, then validates every row against the retained frame. It drops the
row before advancing to the next operation. Full receipt endpoint proofs, both
relation directions, counters, raw table scanning and the disposable range index
remain unchanged. Scan limits still charge distinct operations and all other
actual work cumulatively; they do not promise admission for arbitrary history.
