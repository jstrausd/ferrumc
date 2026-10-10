# Invariants: ferrumc-sim

> Rules that must hold for all code in this crate. Violating these is a bug.

## General

- No `unwrap()` or `expect()` outside `#[cfg(test)]`.
- No unbounded channels or allocations from untrusted input.
- All public items have rustdoc.
- Error types classify the failure mode.

## Crate-Specific

- Inputs are applied **only** at tick boundaries (inside `SimShard::run_tick`),
  never on `enqueue`. Enqueue must not mutate shard state.
- The tick counter advances by exactly one per `TickCoordinator::advance`. There
  are **no catch-up ticks**: lag is reported, never replayed.
- The simulation holds no sockets and no database handles, and reads no wall
  clock in its step API. Stepping is fully deterministic.
- The shard inbox is bounded. When full it rejects with `SimError::InboxFull`;
  it never blocks and never silently drops.
- Player state is held in ordered containers so output ordering is deterministic
  for identical input sequences.
- Accepted regular player placement/break refreshes at most four resident
  cardinal fence neighbors per changed primary/extra cell. Only the arm facing
  that cell changes; waterlogging and unrelated arms remain intact. Unchanged
  results add no secondary journal/output, and rejected or no-op writes initiate
  no refresh. Secondary edits use the existing journal/persist funnel and carry
  no additional player acknowledgement. There is no recursive update queue.
- Exact writes and region/undo operations retain their existing no-physics
  contract. No neighbor chunk is force-loaded by fence refresh; cross-shard and
  load-time reconciliation remain separate. Source fences must be in the pinned
  fence-family facts. Fresh placement queries the neighbour's face towards the
  fence; refresh at `neighbor = center + direction` passes `direction` as the
  changed block's face towards that fence, and replaces only the fence's
  opposite arm.
- Fresh placement and secondary refresh use the pure pinned family, gate-axis
  and 33 connection-exception rules with `full_face_support=false` for fences,
  gates and exceptions. All wooden types, including crimson/warped, share a
  family; Nether-Brick is separate. Gate connections follow perpendicular
  facing and do not depend on open/in_wall/powered. Ordinary blocks retain the
  existing coarse bounding-box fallback as an explicit unresolved seam; it is
  never supplied as a proven FULL support-shape fact to the pure rule or allowed
  to override special-case rejection. Unknown sources/states and vertical faces
  fail closed. The private face-less legacy query requires agreement on both
  horizontal axes; production fence consumers use the directional override.
- The reviewed reference is Paper 1.21.8 Build 60, commit
  `29c8822d90899c89d2689338e81a98f690bcba12`: full `FenceBlock.connectsTo`,
  `isSameFence`, placement/updateShape, `FenceGateBlock.connectsToDirection`,
  support shapes and `Block.isExceptionForConnection`, plus relevant patches.
  Deterministic consumer regression tests do not establish Paper differential
  acceptance or FULL state/face shape parity. Fluid tick scheduling is not
  implemented by preservation of a waterlogged property.
- A chunk is resident in a `LoadedChunkMap` **iff** it holds at least one
  ticket. Acquiring the first ticket runs the load-or-generate flow; releasing
  the last ticket unloads the chunk.
- The simulation owns chunk *data* but never a database handle: a `WorldStore`
  is borrowed per `acquire` call, never stored in the map or shard.
- A metadata map declares one explicit `GenerationBinding` (runtime world /
  dimension and full versioned generator identity). This declaration is not a
  store attestation. Missing metadata is never initialized, guessed or treated
  as a chunk miss. Before any world read/regeneration or resident ticket
  increment, the borrowed store's existing descriptor and generator identity
  must match. Refusal preserves the resident set, tickets, chunks and dirty
  facts. Generator identity is rechecked across awaits and generation; produced
  columns, stored full columns and overlay columns must all match the load key.
- Off-tick verification can create an opaque `VerifiedGenerationBinding` for
  one immutable store's exact `Arc` allocation, scope and identity. It holds
  only a weak allocation identity, never a strong store handle; maps/shards
  retain neither store nor authorization. Synchronous verified capture accepts
  only that exact allocation (including its Arc clones), map scope and
  descriptor. Another same-content store, scope, seed or replacement allocation
  cannot use the token. Custom metadata backends must honor `WorldStore`'s
  immutable baseline contract. No I/O occurs during verified capture.
- Legacy constructors/free-loader aliases are explicitly metadata-free and
  accept only the historic MC 1.21.8 / FerrumC flat revision 1 / Overworld /
  seed-0 generator identity. They refuse metadata-enabled stores, including
  already-resident acquisitions. This explicit historic mode never infers a
  historical descriptor from format-3 data or initializes one.
- Dirty chunks are *collected* for saving (`take_dirty`, and on unload) but
  never persisted here, and no flush policy is implemented — that is the
  caller's concern.
- Metadata overlay capture uses `try_take_persist_dirty_verified` (off-tick
  immutable-store authorization) or async `try_take_persist_dirty_bound` (fresh
  read). Every generation/target-store check and the complete bounded codec
  preflight succeeds before any selected dirty mask or continuation changes.
  The source-compatible `try_take_persist_dirty_bounded` is legacy-only and
  returns `GenerationMetadataUnsupported` on a bound map. Infallible legacy
  `take_dirty`, `take_persist_dirty` and internal scheduler drains are no-ops
  for bound maps, preserving all dirty facts and cursor; an empty result is not
  a metadata flush-success signal. App must consume the bound Result API.
- Production checked overlay capture uses the above selected API: requested
  capacity is clamped to storage's `MAX_SAVE_BATCH` before allocation. All
  selected records are captured immutably before any selected dirty mask clears
  or the deterministic fairness cursor advances. Failure retains every mask,
  item component, chunk and cursor; an unselected tail stays dirty and receives
  its turn before newly re-dirtied lower keys. Legacy `take_persist_dirty` and
  the experimental scheduler's internal variant remain explicitly lossy seams.
- Capture transfers move-owned records only after complete validation, never
  durable ownership. Application driver and writer share the same store Arc;
  writer validates the expected baseline before initial durable saves/retries,
  keeps failed batches and never retargets a captured batch to another store or
  descriptor. Storage checks descriptor existence, not caller intent, so the
  writer's expected-identity check is required. Sim supplies no retry queue or
  flush policy. The experimental scheduler remains legacy-only until its own
  authorized consumer integration.
- Chunk loading selects a stored full `ChunkRecord` when present, otherwise a
  deterministic generated base, and then applies the stored overlay. Overlay
  records must use exactly the current schema; old and future schemas are
  refused as `IncompatiblePreAlphaData`. Overlay sections replace only the
  sections they carry; an empty overlay is a no-op. A non-empty current overlay
  carries the complete block-entity snapshot, so removals persist while imported
  entities survive empty overlays.
- Overlay application uses strict `try_apply_to_chunk`. A malformed entity
  rejects the entire admission without adding a resident chunk or ticket, and
  no load path rewrites corrupt data as empty/default contents.
- Every load key must describe a complete 16×16 column in the i32 block domain
  before storage reads or generation. Full-record and overlay positions must
  match that key even when a custom `WorldStore` bypasses backend validation.
- Chest exchanges stage a private candidate and keep authoritative slots/cursor
  unchanged until the complete candidate passes checked item/entity persistence.
  The current entity is checked before cloning. An invalid cursor or candidate
  beyond the aggregate 16 KiB entity cap returns the original cursor and current
  snapshot without changing dirty masks. An invalid current chest refuses the
  call, preserving the caller's existing cursor ownership.
- Resident chunks and their tickets live in ordered containers (`BTreeMap`), so
  the resident set, its iteration order, and dirty batches are deterministic for
  identical acquire/release/mutate sequences.
- Logical ownership regions are fixed, world/dimension-scoped 8×8 chunk cells.
  Every typed block/chunk coordinate partitions to exactly one cell; arbitrary
  `ShardPos` values that cannot describe a complete cell are rejected.
- `ShardId` canonically includes world, dimension, and `ShardPos`; each id names
  exactly one logical region. Workers/endpoints are separate runtime concepts
  and may process or route multiple shard ids.
- A logical region may have at most one ownership claim.
- Shard lifecycle is strictly `Created -> Active -> Draining -> Stopped`.
  Rejected transitions leave the prior state unchanged.
- The scheduler defaults to the existing one-shard inline path. Its multi-shard
  worker mode is crate-internal, explicitly selected, and not wired into the
  application.
- A shadow tick visits each runnable shard exactly once in canonical `ShardId`
  order. Persistent workers receive owned, disjoint shard batches through
  capacity-one command/result channels; every shard is returned before the next
  tick, and worker completion order never changes output publication order.
- Only `Active` shards accept new scheduler input. `Draining` shards continue
  ticking admitted work, reject new input, and cannot become `Stopped` until
  their tick work is quiescent.
- A worker failure after dispatch permanently poisons that scheduler. A partial
  tick is fail-stop and must never be retried.
- Cross-shard workers emit destination-only intents; only the scheduler may
  stamp their source, source-local sequence, and completed tick. Workers never
  access or send directly to another shard.
- Accepted cross-shard envelopes produced in tick N apply exactly once as a
  separate prefix to the destination's tick N+1 execution. They never compete
  with the ordinary shard inbox. The prefix can enter `SimShard` only through
  an unforgeable scheduler-owned tick capability; sibling modules have no
  constructor or mid-tick mutation path.
- Central admission is bounded (1,024 envelopes by default), nonblocking, and
  reject-newest after sorting by destination `ShardId`, source `ShardId`, then
  source-local FIFO sequence. Rejections return the intact owned envelope.
- Boundary preparation is non-mutating. Tick overflow or pre-dispatch failure
  leaves ready envelopes queued; a successful tick commits exactly the
  prepared scheduler metadata identities, never floating-point payload
  equality. A draining destination cannot stop while an admitted envelope still
  targets it.
- Before a multi-shard tick advances, every resident full `ChunkKey`
  (world/dimension/typed chunk position) must occur in exactly one registered
  shard container. The scan includes clean chunks, so aliases cannot become
  competing persist records on a later tick; conflicts leave tick, inbox, and
  dirty state unchanged. Spatial region membership is not enforced by this
  packet; unique out-of-region residency remains valid until that separate
  architecture step lands.
- A successful scheduler tick transfers nonempty per-shard
  `ChunkOverlayRecord` batches in canonical `ShardId`/`ChunkPos` order only
  after every fallible worker and cross-shard phase succeeds. Each batch is
  capped at storage's `MAX_SAVE_BATCH` (4,096); a canonical overflow tail stays
  persist-dirty and is emitted by a later tick without blocking or loss. A
  deterministic per-map continuation cursor visits that tail before wrapping,
  so repeatedly re-dirtying lower chunk keys cannot starve it.
- Persist output is collected from every registered lifecycle state, including
  a stopped shard with a deferred tail. Clearing a dirty mask transfers
  ownership only into the move-owned scheduler outcome; the simulation still
  holds no database handle and mutation-journal output remains a separate
  bounded seam.
