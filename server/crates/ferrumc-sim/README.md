# ferrumc-sim

Tick coordinator, shard workers, entity systems, chunk tickets, plugin event dispatch.

## Invariants

See `INVARIANTS.md` in this directory.

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
