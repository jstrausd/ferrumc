# ferrumc-sim

Deterministic ticks, shard ownership, player state, chunk residency and optional
bounded non-player registrations and opt-in receipt-confirmed native inventory/item transfers.

## Invariants

See `INVARIANTS.md` in this directory.

## Non-player registration producer

`EntityStore::new(owner, nonzero_capacity)` owns bounded base registrations for
one canonical world/dimension/8×8 chunk owner. Its slot ceiling is at most
65,536, with lazy growth and no unbounded default. Supplied UUIDs, wire ids and
validated registry entity types stay distinct. Both identity indexes and the
fixed 64 intrusive chunk buckets update together; chunk queries borrow only
their occupants and support iterator budgets through `take`/`skip`.

`EntityTransform::try_new` rejects non-finite data and explicit `FerrumC` position
caps (x/z ±30,000,000; y ±20,000,000). It floors negative chunk positions and
normalizes finite yaw/pitch using the reviewed original rotation path. These
resource caps are stricter than original position setters and do not implement
build height, velocity, collisions or movement physics.

`SimShard::initialize_entities` enables one optional store. Read-only
`entities()` cannot mutate it. `run_tick_with_entity_work` creates a checked
`EntityTickContext`, performs synchronous entity work, then drains the ordinary
FIFO tick. Spawn/move require owner-local resident target chunks; no chunk is
force-loaded. Missing initialization refuses before running work or draining
inputs. Each entry also rechecks the current map's world/dimension against the
store owner; replacing the public chunk map with another scope refuses before
work or inbox mutation. Individual refused operations preserve all indexes; the closure is not
one transaction. Existing `PlayerState` owns players, and `minecraft:player`
cannot enter this second store.

Opaque generational handles use a weak incarnation, so stale/reused slots and
recreated or cloned stores reject old handles. Generation never wraps: an
exhausted slot retires permanently. Clone gives a fresh snapshot incarnation,
not permission to activate duplicate global identities. Lookups, transform
updates and borrowed queries allocate no heap object; spawn can allocate within
the explicit cap. Standard tree allocation exhaustion remains process failure.

Registrations survive last-ticket release. Resident memory, entity visibility,
tracking and ticking are separate states; only the first is currently checked.
There is no app/scheduler entity consumer, durable entity save driver, UUID/wire
allocator, cross-shard transfer, living payload, AI or packet/plugin event output.
The reviewed Paper 1.21.8 Build 60 lookup/section/rotation/lifecycle sources and
these deliberate boundaries are recorded in
`docs/agent-tasks/SIM-03_2_ENTITY_OWNERSHIP.md`. Source review and producer tests
do not establish complete entity behavior or a live Paper comparison.

## Owned item state and native snapshots

`initialize_entities_with_item_budget` configures the existing owner slots with
an explicit encoded-body allowance up to 16 MiB. Legacy constructors retain a
zero item budget. Base-only item registrations are placeholders and real item
operations refuse them as `MissingItemState`; they never acquire an empty
default payload. Concrete admission and all payload mutation use the existing
`EntityTickContext`. Refused spawn/restore/replace returns every incoming owned
item unchanged. Legacy remove refuses payload slots; `remove_owned` returns
their base and entire state, including after ticket unload. Direct legacy
store transform mutation refuses payload slots; the checked context moves the
same slot without transferring item ownership.

Native FCIE schema 1 uses a 144-byte header and the checked lossless FCIS stack
codec. The whole body is at most 256 KiB, so even an otherwise valid FCIS stack
can be refused by the tighter native limit. Aggregate usage counts complete
native bodies; it does not claim equal decoded heap or RSS. Motion/bob admit
finite values and retain IEEE bits, including signed zero. Age, pickup delay
and health retain their full signed i32 runtime values. Pickup target and
thrower are independent UUIDs. Native restore checks outer key, scope, type,
chunk and canonical transform before FCIS allocation, then requires an explicit
new wire ID from the caller. Capture performs no I/O, dirty clear or save ack.

Checked item/store snapshots validate payloads before cloning. Store/SimShard
Clone remains source compatible and validates the entire item sum first; an
impossible private invariant violation is fail-stop in that infallible trait,
while `try_snapshot`/capture return typed errors. Copies have fresh store
incarnations and do not grant permission for competing active owners.

Active/inactive counter projections preserve Java wrapping, sentinels and the
exact signed despawn comparison. They return only Keep/Due. Cancellation age
reset and owned removal are separate calls. These helpers supply no physics,
merge, pickup, activation decision, event dispatch, tracking or gameplay wiring;
complete active ticks must interleave delay and age changes correctly around
physics/merge. Unloaded/offline items receive no automatic or wall-clock step.
See `docs/agent-tasks/SIM-03_3_ITEM_ENTITY_STATE.md` for the pinned reference,
native-policy deviations and acceptance evidence.

## Receipt-confirmed native player/item consumer (DUR-04.1)

`initialize_player_ownership(binding, limits)` explicitly selects a single
native authority only while the shard's player/entity owners are empty. The
actual `PlayerState` then owns all 46 indexed window-0 slots, the cursor and
selected hotbar alongside position/look/mode. `NativePlayerSnapshot` reads and
writes the existing FCPS3/schema3 format exactly: 41-byte header and 47 framed
lossless FCIS stacks. It refuses unsupported schemas, invalid shapes, nonfinite
positions/look, invalid selection, oversized frames and trailing bytes. FCPS1/2
migration remains a separate App consumer.

`OwnershipStoreBinding` retains only weak exact backend/incarnation identities.
Off-tick `read_player`, `read_entity` and coherent `read_pair` produce sealed
admissions from real DUR-03 ownership reads. `OwnedPlayerInput` enters a bounded
count/byte inbox; refusal returns the entire input. The concrete
`run_tick_with_player_ownership` applies completions first, then admitted action
FIFO. It emits move-owned `PlayerOwnershipStorageWork` for the external driver,
which executes the exact real CAS/pair request, receipt clarification and current
read. Sim performs no database I/O during this boundary.

Bootstrap, ordinary slot/cursor left/right clicks, autosave, selected-hand
DropOne/All, partial/full pickup and fresh item spawn publish only after matching
receipt/current evidence. The reservation protects the full player and one
whole entity owner. Before confirmation the actual player source and item
visibility remain unchanged; a definitive fresh-spawn conflict returns the
whole original caller proposal through `ItemSpawnRefused`. Deferred movement,
look, mode and selection apply afterwards at checked new generations.
Disconnect keeps native state and pending work; rejoin invalidates the old
player admission. Dirty or unresolved work prevents Stop.

Each pending mutation has at most one outstanding Work/Completion buffer,
including a refused completion retained by the caller. A weak lease tracks that
buffer only. Losing it allows readonly reconciliation, never proof of a worker
abort. Receipt absence retains reservations. Only a real no-start Busy/history
refusal permits exact retry; superseded historical receipts quarantine/resync.
Version-1 FCPQ checkpoints are checked data and cannot forge workers,
expectations or live handles. Receipt-positive recovery uses fresh current reads;
a pair is read coherently. Snapshot/checkpoint capture is bounded per call;
external retention of repeated caller-owned copies is outside Sim retention caps.

Limits cap players, checked native player bytes, pending count/charged bytes and
combined input/completion count/charged bytes. Pending charge is checked before
materializing candidate/request clones:
`512 + 2*before_record_bytes + 5*after_record_bytes + 149`. The fixed 512 is a
logical metadata allowance, including the one private receipt-box pointer;
boxing moves the existing receipt and creates no second receipt. All charges
are encoded-representation/count limits, not complete decoded heap/RSS limits.
A maximally allowed Storage pair may exceed Sim's stricter 8 MiB pending cap and
is refused. Owned-mode clones are inactive snapshots; legacy input, entity
writer, container and mutable-chunk-map entry points cannot bypass authority.
Use checked owned chunk acquire/release for residency changes.

The consumer preserves exact component representations and conservatively
resolves typed stack caps. Removed/opaque capacity components 1..3 refuse; it
does not implement effective default-component normalization. Recipe output and
all armor clicks refuse. Pickup considers selected matching hotbar, occupied
matching offhand, remaining hotbar/main, then free hotbar/main; excluded recipe,
crafting, armor, cursor and empty offhand are preserved. The current proximity
check is a conservative envelope rather than native collision/tracking parity.
Standing drops preserve sampled motion and start with pickup delay 40, age 0,
health 5 and independent thrower. Synthetic RNG streams and standing pose are
not true native constructor/player seeding or pose differential acceptance.

These consumers are opt-in and **not wired into the live App**. The App currently
uses legacy Format4 storage; fresh Format6 ownership selection and explicit
old-data migration require DUR-05. No owned automatic item timer, movement,
collision, merge/despawn, loaded activation, packets or Java events run here.
Drops cannot mature their delay until SRV-02.1/DUR-04.2. Real Memory/redb consumer
and codec tests establish this isolated authority contract, not live Survival,
Paper plugins or a complete server replacement. The pinned reference, source
manifest, author evidence and immediate integration plan are in
`docs/agent-tasks/DUR-04_1_PLAYER_ENTITY_CONSUMER.md`.

## Generation binding and persistence capture

Metadata consumers declare a `GenerationBinding` with an explicit storage scope
and `GenerationIdentity`, then construct `LoadedChunkMap::with_generation` or
`SimShard::with_generation`. The declaration does not initialize metadata or
attest that a store accepted it. Every acquire, including an already-resident
ticket increment, verifies the generator and existing persisted descriptor
before world reads or mutation. Missing, unsupported, corrupt or incompatible
metadata refuses admission. A regenerated base receives the same strict overlay
composition as a stored full base; the generator's returned column is checked.

For the synchronous tick path, bootstrap calls
`binding.verify_store(&Arc<dyn WorldStore>).await` off-tick. The opaque
`VerifiedGenerationBinding` binds the exact immutable descriptor, scope and
store allocation, retaining only a weak identity. Production capture uses
`map.try_take_persist_dirty_verified(&store, &verified, tick, capacity)` with no
storage I/O. Driver and writer must share clones of the same store `Arc`.
Another same-content store cannot reuse the authorization; a dropped store is
not kept alive. Custom backends must obey `WorldStore`'s immutable metadata
contract.

`try_take_persist_dirty_bound(&dyn WorldStore, tick, capacity).await` instead
performs a fresh metadata read. Both checked paths clamp the record count to
storage's batch cap and preflight every selected overlay before clearing any
dirty mask or advancing the fair continuation. Any authorization, metadata or
codec failure preserves the complete batch's dirty ownership and cursor.
Returned records still require application-owned durable save/retry. The writer
must validate the expected binding before initial saves/retries, retain failed
batches, and never retarget a batch to another store or descriptor.

Chunk loading strictly applies overlays and refuses malformed entity contents.
Chest exchanges stage their result and enforce the checked 16 KiB aggregate
entity limit before committing, preserving the original cursor on refusal.

The existing application remains on explicit metadata-free, format-4 legacy
mode until the separate App activation ticket. `new`, `with_schema_version` and
`load_or_generate` are source-compatible aliases for the named legacy paths.
Legacy acquire accepts only the explicit historic Minecraft 1.21.8 / `FerrumC`
flat revision 1 / Overworld / seed-0 baseline and refuses metadata stores.
`try_take_persist_dirty_bounded` remains the checked legacy capture API; it
returns `GenerationMetadataUnsupported` on a bound map. The infallible
`take_dirty`, `take_persist_dirty` and experimental scheduler drain are no-ops
on bound maps, preserving all masks/cursors. Their empty results are **not**
metadata flush-success signals. The lossy legacy drains do not establish
component durability.

The reviewed reference is Paper 1.21.8 Build 60, commit
`29c8822d90899c89d2689338e81a98f690bcba12`, including applied chunk-status tasks
and the active Moonrise load/generation path. `FerrumC`'s metadata and overlay
format are its own contract. This stage adds safe generator admission, not
Vanilla terrain, feature/population stages, playable Survival or Paper runtime
parity. See `docs/agent-tasks/WG-003_SIM_GENERATION.md` for the consumer contract,
sources and verification.
