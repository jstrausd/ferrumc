# ferrumc-sim

Deterministic ticks, shard ownership, player state, chunk residency and optional
bounded non-player registrations.

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
There is no app/scheduler entity consumer, entity persistence, UUID/wire allocator,
cross-shard transfer, living/item payload, AI or packet/plugin event output.
The reviewed Paper 1.21.8 Build 60 lookup/section/rotation/lifecycle sources and
these deliberate boundaries are recorded in
`docs/agent-tasks/SIM-03_2_ENTITY_OWNERSHIP.md`. Source review and producer tests
do not establish complete entity behavior or a live Paper comparison.

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

The existing application remains on explicit metadata-free, format-3 legacy
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
