# ferrumc-sim

Tick coordinator, shard workers, entity systems, chunk tickets, plugin event dispatch.

## Invariants

See `INVARIANTS.md` in this directory.

Production persistence consumers use
`LoadedChunkMap::try_take_persist_dirty_bounded(tick, capacity)`. It clamps the
requested record count to storage's batch cap and captures every selected
overlay before clearing any dirty mask or advancing its fair continuation.
Capture errors preserve the whole selected batch's dirty ownership. Returned
records still require a durable save/retry path owned by the application.

Chunk loading strictly applies overlays and refuses malformed entity contents.
Chest exchanges stage their result and enforce the checked 16 KiB aggregate
entity limit before committing, preserving the original cursor on refusal.

The older `take_persist_dirty` method and the experimental multi-shard
scheduler's bounded legacy variant remain source-compatible, lossy entity
capture paths. They do not establish component durability.
