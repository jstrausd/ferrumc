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
- The owned fallible load APIs share the existing full-base/overlay composer.
  Natural CPU generation runs only in an admitted blocking task; producer errors
  preserve their World classification and never substitute flat/empty terrain.
  One shared `ChunkGenerationExecutor` bounds submitted/running generation tasks
  (hard cap 64, caller-selected normal cap one or two). Full admission returns
  `ChunkGenerationBusy` immediately with no queued waiter or submitted task.
  Task ownership retains its credit after caller cancellation until worker exit.
  Full stored bases and already-resident tickets consume no generation credit.
  Identity is checked across metadata/storage awaits and inside the worker before
  and after successful generation. Every failure leaves the failing column's
  residency, tickets, dirty masks and capture cursor unchanged. Spawn batches
  preserve earlier successful column acquisitions on a later column failure.
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
- Optional non-player registrations belong to one `EntityStore` with an
  explicit nonzero slot ceiling no greater than 65,536. There is no unbounded
  default. Slots grow lazily and retired slots count against that ceiling.
  UUID and wire indexes never exceed the live count; fixed 64 chunk buckets use
  intrusive slot links, so chunk queries do not scan unrelated registrations.
- `EntityUuid`, wire `EntityId`, registered `EntityTypeId` and the opaque local
  `EntityHandle` are separate identities. Spawn refuses duplicate UUID/wire ids
  locally, a wrong world/dimension/8x8 owner, and the player type. Existing
  `PlayerState` remains the only player-state owner. UUID/wire allocation and
  duplicate prevention across stores are not supplied by this producer.
- Every entity transform is finite, with explicit FerrumC rejection caps of
  x/z ±30,000,000 and y ±20,000,000 blocks. Those caps are not original setter
  parity or world build height. Chunk assignment floors negative coordinates.
  Finite yaw is `% 360` and pitch is `(pitch % 360).clamp(-90, 90)`.
- Handles bind owner, slot, generation and a weak store incarnation. Remove
  invalidates old generations; maximum generation permanently retires a slot
  rather than wrapping. Recreated/cloned stores reject source handles. Clone
  copies bounded registrations as a snapshot with a fresh incarnation; it is
  not authorization to activate competing global UUID/wire owners.
- Entity operations preflight before mutation. Returned errors preserve
  records, all identity/spatial indexes and handle validity. Successful lookups,
  borrowed chunk queries and transform updates allocate no heap object. Slot
  reservation is fallible before commit; standard BTreeMap allocation failure
  terminates the process and is not advertised as a recoverable Result.
- `SimShard` exposes registrations read-only. Initialization is explicit and
  once-only. The mutable `EntityTickContext` exists only inside
  `run_tick_with_entity_work`; spawn/move validate scope/owner/handle before
  requiring a ticket-resident target. Every entity tick entry first verifies
  that the current loaded map's world/dimension still equals the store owner,
  even after a caller replaces the map through `loaded_chunks_mut`. A missing
  store or mismatched map refuses before closure execution or ordinary inbox
  drain, preserving registrations, handles and resident facts. The closure executes before the ordinary
  tick; each operation is atomic, but the whole closure is not a transaction.
- Registration, chunk residency, accessible/tracked status and entity-ticking
  status are distinct. Last-ticket release preserves registrations; subsequent
  spawn/move into the unloaded target refuses, while remove and migration to a
  different resident chunk remain available. No chunk is force-loaded. There
  is no automatic entity tick, activation, AI, collision, storage driver,
  plugin event, tracking output, app/scheduler consumer or cross-shard transfer.
  Removal returns base data, never a durable delete/save acknowledgement.
- Concrete item state lives directly in the existing entity owner slot, with
  one owned stack and sealed finite properties; no payload sidecar, second
  owner, raw mutable stack/patch access or generic component framework exists.
  Legacy item-type registrations remain explicit placeholders; real consumers
  return `MissingItemState`, never an EMPTY/default fallback.
- Existing store constructors admit no item bytes. Opt-in configuration is
  explicit and at most 16 MiB of complete native bodies per owner. Every native
  FCIE body is at most 262,144 bytes including its fixed 144-byte header and
  entire FCIS stack. Valid larger FCIS stacks are refused unchanged. The sum
  of checked admitted body lengths is updated once at spawn/replace/remove;
  scalar counters, properties and transforms do not change that sum. This is
  an encoded-byte bound, not an RSS guarantee.
- New item spawn/restore/replace/property/counter/remove mutations require the
  existing non-escaping `EntityTickContext`. Legacy store transform updates
  refuse payload slots; context updates preserve that same owning slot.
  Legacy remove refuses payload slots before any detach/index/generation/budget
  change; owned remove returns the base and complete payload and remains legal
  after chunk unload. All owned-input refusals return their complete input
  without normalization or cloning. No payload is silently disposed by an
  old base-data removal API.
- Snapshot/capture validates semantic FCIS, finite properties, canonical native
  basis, cached lengths and aggregate accounting before payload clone or
  publication. Whole-store snapshot revalidates every payload before its first
  clone; single-item capture checks its entire row and cached owner aggregate.
  Store/SimShard Clone copies only sealed admitted states with fresh incarnation;
  impossible private invariant corruption is fail-stop in that existing
  infallible trait. Public try_snapshot/capture return typed errors instead.
- Native item storage preserves signed i32 age/delay/health, all finite motion
  magnitudes and signed-zero motion/bob bits. It deliberately does not apply
  Vanilla NBT short truncation, motion-above-ten sanitization or corruption
  defaults. Target and thrower UUIDs are independent. FCIE contains no wire ID
  or local handle. Decode checks outer schema/key/scope/UUID/type/chunk and
  canonical finite header before stack allocation; lossless FCIS bytes must
  re-encode identically. Restore requires a newly supplied caller wire ID and
  owner/resident/duplicate/capacity/budget admission before publication.
- Pure active/inactive item counter projections decrement positive delay except
  32767, increment age except -32768 using Java i32 wrapping, and compare the
  resulting age against the exact signed caller rate. They do not independently
  exempt sentinel age from a negative-rate comparison, cause health death,
  remove an item or emit events. Keep/Due, cancellation age reset and owned
  removal are separate. Full active ticks must preserve physics/merge between
  the delay and age phases. No automatic activation, once-per-tick claim,
  unloaded/offline step or wall-clock catch-up is implemented by these pure
  projections. The explicitly opted-in DUR-04.2a consumer below owns checked ticks.
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

## Native player/item ownership mode (DUR-04.1)

- Explicit empty-owner initialization binds the current shard, exact immutable
  generation/scope and one weak backend allocation. Actual PlayerState owns all
  46 native slots, cursor and selection; opaque handles bind Sim incarnation,
  UUID and nonwrapping admission generation. Clone is inactive SnapshotOnly.
- Legacy GameInputs (including scheduler boundary input), generic entity work,
  legacy container clicks and raw mutable chunk-map access cannot mutate this
  mode. The legacy infallible mutable-map accessor panics on programmer misuse;
  checked owned acquire/release preserve scope and actual reservations.
- FCPS3 admission validates fixed grammar, 47 complete FCIS frame extents,
  finite/range/selection/shape and aggregate bounds before nested decoding.
  Other schemas are not silently migrated or defaulted. Only a real never-seen
  miss bootstraps empty Survival; a tombstone never grants bootstrap authority.
- Queued count/encoded-byte refusal returns the whole input. Pending count,
  exact checked `512 + 2*before + 5*after + 149` charge and all resource gates
  precede candidate/request component clones. The fixed allowance is logical
  metadata, including the sole receipt-box pointer, not full allocator/RSS
  accounting. Temporary encodings and externally retained snapshots/checkpoints
  are bounded per call, not claimed as globally bounded caller memory.
- One player and the whole entity owner remain reserved until genuine matching
  evidence. Every pending mutation has at most one outstanding move-owned
  Work/Completion buffer. Its weak lease proves buffer lifetime only. Retained
  refused completions block another dispatch. Expired buffer permits readonly
  reconciliation; receipt None never proves worker abort or releases items.
- Only actual no-start Busy/history-capacity evidence allows exact retry. No
  RNG/event/item selection repeats under the same query. Commit uncertainty or
  current-read failure retains exact pending authority. Foreign/stale/different
  query/generation/lease completions cannot publish. Receipt scope, full identity,
  keys, before/after stamps, native bytes, affected chunk epochs and current real
  resources must agree; a historical superseded receipt quarantines/resyncs.
- No database read/write runs in the ownership tick. Off-tick evidence factories
  are concrete and sealed; caller values/booleans cannot forge completion.
  DUR-05 must serialize the real writer/driver and atomically replace App's
  connection inventory. Evidence is a real read boundary, not distributed
  exclusion of arbitrary independent backend writers after that read.
- Confirmed publication changes player/item together before native intents.
  A refused fresh spawn refunds the entire caller-owned proposal; a refused
  player drop retains the actual inventory and does not refund a second copy.
  Full pickup publishes removal only after real tombstone confirmation. Partial
  pickup preserves exact remainder components and normalizes negative delay to
  zero in the confirmed postimage. Stats/take carry original native counts
  separately from actual moved quantities.
- Pending protects the full player postimage. Position/look/mode/selection
  coalesce until completion and apply at preflighted new generations. Disconnect
  retains data/work; rejoin invalidates old local admission. Draining admits
  clarification/autosave, and Stop requires no dirty/pending/queued/boundary work.
- Pickup capacity/default-component policy, collision/pose/RNG parity and all
  armor clicks remain conservative explicit limits. Delay40 drops have no owned
  timer in this ticket. Physics, merge/despawn, activation/tracking, packet/App
  and Java-event consumers require immediate follow-ups; this is not live
  Survival or complete Paper compatibility.


## Owned resident item counters and durability (DUR-04.2a / SRV-02.2)

- Explicit initialization on the empty DUR-04.1 owner declares positive caps and
  its actual last-completed logical Tick. Only the checked exact successor is
  accepted; duplicates, gaps and Tick overflow refuse before scratch/inbox drain.
  Terminal owner publication generation `u64::MAX` similarly refuses the entire
  next logical boundary before scratch, counters, cursors or Tick advance, even
  with clean items or no save IDs. `MAX-1` remains usable. Completion-only recovery
  still checks each actual further publication generation before drain; it cannot
  call an impossible acknowledgement quiescent or confirmed.
- A readonly bounded scalar plan overlays the genuine completion prefix,
  publication/defer and concrete FIFO admission before the first real drain.
  Restore, accepted/refused rejoin, bootstrap, reservations, checked generations,
  body charges and Stop use the future state. Player native validity/position is
  checked from sealed genuine reads off-tick. No components/native body are cloned
  for timer planning. Private slot Vec capacity is fallibly reserved before drain
  for bounded future publications; no identities/properties are published there.
- Borrowed stack sizing in input admission, inventory plans and counter boundary
  Spawn/Drop preflight uses the checked Items `persisted_stack_size` authority,
  without FCIS serialization or component cloning. InventoryPlan comparison
  retains its private empty/item guard after all borrowed sources were fully
  validated, then uses exact ordered/bitwise `same_stack_representation`.
  Selected materialization and genuine persistence still encode their bodies.
- Every eligible resident live item receives exactly one scalar step in each
  accepted Tick, up to 65,536 items. Unloaded items receive zero and no offline
  catch-up. A fully accepted Stop has no subsequent counter steps. Save scanning
  alone defaults to 64 slots and can be configured to at most 256; it never samples
  or limits the counter phase. The complete counter phase is bounded O(N).
- Delay decrements only when positive and not 32767. Age Java-wraps unless -32768;
  due compares the resulting signed age with the actual signed rate, including
  negative rates. Health is not timer death. This explicit VanillaNoPlugins stage
  marks due items ineligible for gameplay and prioritizes genuine receipt-backed
  Entity CAS delete. It supplies no fictitious Java event/cancel/activation/physics.
- Each private live slot owns both its current generation and last-confirmed
  native FCIE body/stamp/saved generation/Tick. Confirmed bodies have a separate
  positive at-most-16 MiB cap from the existing at-most-16 MiB live body cap.
  Aggregate confirmed accounting precedes confirmed native decoding and any
  snapshot clone. No per-counter-step body encoding or durable CAS runs.
- Exact scalar scratch caps are at most 65,569 descriptors and 9 MiB including
  actual Rust `size_of` metadata. Checked multiplication and fallible allocation
  precede drain; retained scratch is reused. Encoded-body and metadata charges
  do not claim equal decoded heap, allocator or RSS usage.
- At most one Entity operation reserves the whole owner structure. Its visible
  body and request postimage stay frozen; unrelated items may change only scalar
  counters, leaving indexes, stack lengths and Prepared.used unchanged. The one
  private original/postimage projection records at most 1024 actual logical claims.
  Claim 1025 refuses the entire next Tick with CounterRecoveryRequired. Lost reply
  or receipt None retains pending authority; CasConflict quarantines with the
  original scalar projection and never publishes into the reserved visible body.
- Genuine confirmed Rest/Spawn/Save publication installs captured confirmed bytes
  and saved generation, then publishes only its checked bounded defer. The actual
  publication boundary Tick names last save; completion-only recovery uses the
  existing completed Tick. Full removal makes no defer publication. Missing
  preflighted Rest/Spawn/original bodies fail-stop, never silently lose a step.
- Pickup uses confirmed native bytes as the exact DB beforeimage and current
  live counters/stack for the conserving afterimage. Pure borrowed inventory
  planning shares the actual admission rule. Partial pickup normalizes negative
  delay only in its confirmed rest; definitive refusal retains original delay.
  No-op Entity CAS compares schema/chunk/full native bytes, preserves revision
  and requires empty affected epochs; genuine changed CAS checks incremented epochs.
- Save selection admits at most one job per boundary, at most 2 MiB logical
  `661 + 2*before + 5*after` charge within the shared at-most-8 MiB pending cap.
  Two slot cursors advance after each examined slot, three priority turns to one
  normal turn, and freeze while Entity pending exists. The identical query survives
  Busy/history refusal and uncertainty; retained refused completion leases also
  block same-boundary retry until the caller drops that intact input.
- A concrete Driver refill owns at most 32 IDs/512 raw ID bytes, bound to the real
  owner incarnation. Local duplicate/cap refusal returns the whole refill.
  An ID is consumed only on actual pending admission; empty pool emits at most
  one SaveIdsNeeded intent per boundary and does not stop timers. The Driver owns
  UUID authority. Confirmed does not mean an invented globally unique Sim counter.
- Save period 100 means due after 100 logical Ticks, not a five-second durability
  guarantee: 4096 dirty items with one successful job per boundary require at
  least 4096 boundaries plus scanning/worker latency. Unload and controlled Drain
  retain priority until dirty deferred state is genuinely saved. A Driver seeking
  quiescence must unload or use completion-only recovery/flush; resident logical
  ticks can continue making timers dirty. Stop needs true confirmed quiescence.
- Opt-in refuses every ordinary GameInput at enqueue with OwnershipInputRequired;
  owned Move/Mode/Select remain the concrete player inputs. Infallible legacy Tick
  and scheduler entry fail-stop rather than bypass Tick authority. App Format6,
  regular block/command consumers, physics/activation/merge/contact/tracking,
  Java events/plugins and live Survival/Paper differential acceptance remain
  separate DUR-05 and gameplay contracts.

## Reserved immutable item publications (SIM-03.4)

- Publication mode is separately selected once on an empty actual ownership
  owner. Snapshot clones receive neither pending authority nor publication
  outbox and cannot initialize, peek or acknowledge a competing producer.
- Positive caps cover pending plus retained groups (at most 256), total logical
  encoded charge (at most 8 MiB), and one preparation (at most 1 MiB). Charge is
  256 scalar accounting bytes plus exact borrowed FCIS stack size, or 256 for
  removal; this is neither a wire format nor decoded heap/RSS accounting.
- The complete outer FIFO is fallibly reserved before activation. Borrowed
  model/charge/sequence checks precede the additional bounded stack clone and
  Workrequest. Existing Item/NBT deepclones and existing Arc/BTreeMap allocations
  retain their infallible allocator boundary; there is no all-OOM Result claim.
- Pending owns one immutable staged body and a group/byte/sequence credit.
  Confirm moves the same charge into retained accounting without another body
  clone or queue allocation. Only definitive abort releases pending credit;
  Busy, uncertainty, absent receipt, refresh and quarantine retain it.
- Publication sequence is assigned in actual confirm/restore order, not at
  admission. Checked `last + reserved + 1` prevents later overflow. Abort consumes
  no sequence, MAX may publish once, and retries never mint duplicate facts.
- Whole scope/generation, actual weak store/slot incarnation, slot generation,
  UUID, wire ID and type bind each immutable instance. Removed instances remain
  bound after slot reuse. Group/postimage/origin/take have no public constructor,
  Clone, mutable body access or snapshot-to-publication factory.
- Confirmed postimage scalars are captured directly after counterconfirmation
  and before legacy intents/deferred player work/later counter boundary. The
  staged unchanged stack moves with them. Restore uses genuine checked native
  Store/Generation admission, with no fabricated receipt. Counter-only and
  no-op autosaves produce no group or additional publication stack clone.
- Optional Take precedes Changed/Removed logically and retains original animation
  count independently from genuinely moved quantity. Collector captures the
  validated original player owner/admission before Workrequest. Rejoin never
  refreshes it. `matches_owned_player` compares that original admission only;
  connectivity/current Session/scope/generation and viewer-specific collector
  wire IDs require the separate Session admission contract.
- Peek borrows the retained FIFO head. Ack accepts only its actual-owner,
  full-instance/sequence ticket and releases exactly that group once. Refused,
  stale, reordered or foreign output preserves all data/charges. A ticket is
  not socket delivery evidence: Driver must complete whole Session admission
  before ack. Peek/ack remain available during Drain; Stop and scheduler
  quiescence additionally require empty reserved/retained publications.
- Countershadow uses the same scalar charge/admission predicate and genuine
  completion prefix: Confirm retains credit, definitive abort frees it, new
  Restore/Action gates precede further shadow resource mutation. Full outbox
  refuses due-delete save admission without consuming its supplied SaveID;
  unload neither invents removal nor drops retained publication data.
- Ephemeral publications are absent from the unchanged durable checkpoint
  format. Session/driver activation, metadata refusal policy for present AIR or
  trusted opaque inner schemas, socket emission, item physics and live Survival
  remain separate consumer/gameplay tickets.

## Collector admission lifecycle addendum (SIM-03.4a)

- `connected_owned_player_view` borrows actual native PlayerState/owner state,
  checking private owner/admission, complete current chunk/entity generation,
  connected and not stopped. Drain preserves connected output authority.
  Native mode/pose are read without normalization; seed obfuscation uses the
  existing `BiomeZoomSeed::from_world_seed` producer, never caller hash bits.
- Once-only empty-owner opt-in requires actual publication configuration.
  Lifecycle cell count is checked actual player limit plus actual group limit;
  exact outer charge is `size_of::<Vec<Option<CollectorLifecycleCell>>>() +
  count * size_of::<Option<CollectorLifecycleCell>>()`, within a positive
  at-most-one-MiB scalar byte cap. The entire Vec is fallibly reserved before
  activation. Logical charge is not allocator/RSS measurement.
- Cell admission is checked before native Bootstrap/Restore body copies. Staged
  scalar credits move into real pending Bootstrap or immediate Restore admission;
  no-start refusal releases staged work, definitive Bootstrap abort releases its
  reserved cell, uncertainty/quarantine keeps it. Only actual admitted handles
  seal cells. Every rejoin receives its own cell; unacknowledged pinless old
  detach facts remain counted, with no unbounded history or payload sidecar.
- Native Disconnect seals exactly one retained scalar fact after the actual
  admission becomes disconnected. Repeated historical Disconnect uses that
  existing fact without touching a rejoined PlayerState. UUID-only detached
  intents and enqueued Disconnect are not proofs. Facts/views have no public
  factory, Clone, Weak getter or mutation handle.
- Every genuine staged `take:Some` in lifecycle mode carries one original-cell
  pin; admission precedes candidate body copies and Workrequest. Pendinginstall
  reserves it, Confirm moves reserved to retained, definitive release_pending
  and valid actual FIFO head acknowledgement release it exactly once. Busy,
  history refusal, lost reply, absent receipt, refresh, quarantine and detach
  retain it. Reserved plus retained pins never exceed the actual group cap.
- Countershadow copies fixed lifecycle credit scalars and uses the same cell/pin
  transitions over the genuine completion prefix before further admissions.
  Counter scratch includes the actual new scalar resource layout; Tick overflow,
  scratch refusal and scope mismatch preserve queued work and lifecycle credits.
- Historical lifecycle views borrow original cells after rejoin. Only genuine
  detached cells with both source pin counts zero mint exact cell-credit/owner/
  admission/full-generation tickets. Ack revalidates all bindings and zero pins;
  stale, foreign and repeated tickets release nothing. Driver must separately
  prove original Session retirement and zero consumer refs before Sim ack.
  A Sim ticket is no socket or network receipt.
- All unresolved lifecycle cells prevent native Stop and scheduler quiescence.
  Drain permits publication/lifecycle acknowledgements. Clone/SnapshotOnly has
  no runnable cells, detach facts or pins. Durable checkpoint formats, App and
  Session activation, Java events and live Survival remain separate contracts.

## Detached native player release (SIM-03.4b)

- `saved_after_detach` is private PlayerState data set only by Sim while it
  applies the genuine `Confirmed{current: Ok}` completion of an Autosave pending
  of exactly this admission, if the player is then detached. The latest
  confirmation wins. Every newly admitted pending of the player, every deferred
  apply/dirty mutation and every (re)admission publication clears it; an
  uncertain later outcome keeps its pending and therefore blocks release.
- `lifecycle_acked` is set only by a successful `acknowledge_collector_lifecycle`
  for the cell of exactly the current admission; acknowledging an older
  admission's cell never marks a rejoin. Non-admitting publications preserve it,
  only a new admission resets it. Without lifecycle mode there is no cell and
  release refuses (`LifecycleNotInitialized`), never treating "no cell" as
  acknowledged.
- `detached_owned_player_status` and `owned_stop_ready` borrow actual state only
  and allocate nothing. `owned_stop_ready` and the Stop apply share one private
  predicate, so it is true exactly when a Stop enqueued as the only input is
  accepted by the next normal boundary.
- `release_detached_owned_player(original)` takes no caller query and runs only
  between boundaries. Every gate is a borrowed scan before the single mutation:
  live owner (not SnapshotOnly, not stopped), actual admission, detached,
  lifecycle mode, no queued input naming the player, no reserved or retained
  Take pin, no item/chunk (entity owner) reservation, no other pending, this
  admission acknowledged, no other lifecycle cell of the player, clean state and
  `saved_after_detach`. Refusals are typed (`DetachedReleaseRefusal`, or
  `StaleHandle`/`SnapshotOnly`/`Draining`/`NotInitialized`) and change nothing.
  Success removes the PlayerState and its confirmed bytes from player accounting
  in the same step; the receipt carries only PlayerId and freed bytes. One call
  visits at most one player entry, 32 inputs, 32 pendings in each of three
  passes (Take pins, item reservation, other pending), 256 retained groups and
  384 lifecycle cells, 769 entry visits within the contract bound of 833, plus
  one keyed pending lookup per queued Retry/Reconcile, and allocates nothing.
  There is no automatic eviction.
- A bootstrap pending can only exist for a PlayerId without PlayerState, so it
  never coexists with a releasable handle; a queued Bootstrap/Restore/Rejoin
  read of the same PlayerId refuses as a queued input. Chunk tickets carry no
  PlayerId; the only chunk-bound player reservation is the pair/entity pending.
- `ferrumc_sim` re-exports `OwnershipTransferId` and `OwnershipReceiptQuery`
  as plain identity values for consumers without a storage dependency.

## Selected hotbar slot in the connected view (SIM-03.4c)

- `ConnectedOwnedPlayerView::selected` returns the typed `HotbarSlot` (`0..=8`)
  of exactly the viewed admission. Its only source is the owner's private
  `OwnedPlayerInventory::selected`: an accepted `select_hotbar` writes it when
  applied, and every native save materializes `NativePlayerSnapshot::selected`
  from it. Bootstrap, restore and rejoin seed it from the admitted durable
  snapshot. There is no Session-side or second Sim copy.
- The view checks the field once at creation, under the same shared shard
  borrow that keeps it unchanged for the view's lifetime; the accessor is a
  `const` copy without allocation. A field above `8` would refuse view creation
  with `Malformed`, which no admitted writer can produce.
- A selection deferred behind a pending save, or refused (out of range,
  detached, stale or foreign handle), is not visible, exactly like position,
  look and mode. Detached, stale, foreign, stopped and `SnapshotOnly` handles
  still produce no view. The Session held-slot resync and its driver wiring
  remain NET-04.2f.

## Player placement body refusal

- BlockPlace checks every final refined candidate cell, including relocated slab
  merges and extra bed/door cells, before the first mutation. SetBlockExact
  applies the same gate to the plugin-selected final state. Raw non-player
  Command/Plugin/Test exact edits retain force-edit semantics.
- Every present nonspectator player's standing body obstructs placement,
  including the actor. Spectator actors refuse; disconnected owned players are
  excluded. Only earlier valid FIFO coalesced movement prefixes are overlaid
  for this check; future moves do not apply early or reorder existing outputs.
- Geometry uses ferrumc-placement's pinned original collision fixtures. Unknown
  or unsupported shapes/bodies refuse safely. A refusal leaves chunks, block
  entities, dirty facts and journal unchanged and corrects the primary and all
  extra predicted cells. Extra corrections repeat the input's ACK sequence.
- SimShard::enable_placement_completions opts into exactly one internal
  PlayerPlacementResolved per BlockPlace/SetBlockExactTracked, after all action edits
  or corrections. Some carries the actual final primary state and position
  (including relocated merges); every refusal reports the originally requested
  position and None, including absent actors. Default-off preserves legacy output
  collectors. Consumers resolve after-hooks from this event, not previews,
  repeated sequence IDs on ordinary block changes, or extra-cell writes.
- This is standing-player placement obstruction, not full original entity,
  pose, support physics, canSee, survival inventory or plugin-event parity.

## Short-grass replacement admission

- Ordinary player placement may replace an existing short_grass cell with a
  known different held block. The primary and extra-cell replaceability gate
  use the original held state; placing short grass into itself still refuses.
  Final-state body collision is checked before this gate and before every edit,
  so a colliding replacement preserves the existing plant, dirty masks and
  journal. Successful replacement journals the short-grass prior state.
- The App selects clicked-versus-adjacent placement from authoritative state;
  Sim still revalidates the target at the tick. This narrow policy adds no
  general plant, support, survival, pose or Paper plugin-event mechanics.

## Tracked exact placement completion

- Ordinary SetBlockExact world intents use the same player-body collision gate,
  but never emit PlayerPlacementResolved. Only explicit plugin Replace for a
  placement action uses SetBlockExactTracked. A before_interact world intent
  sharing the client's sequence cannot resolve the explicit placement reply.
- Tracked exact refusal and acceptance each emit one opt-in internal outcome;
  ordinary BlockPlace continues to do so. Session treats the tracked input as
  authoritative data and routes by the final block's owner. Its internal
  completion has no client packet or disconnect effect.

## Explicit legacy Survival mining and ground consumer

- Ordinary start/finish/abort mining retains a block preimage, logical tick
  progress and caller tool witness; changing an exact held slot cancels work.
  Finish is necessary unless the initial progress is already instant. Drops and
  chest contents pass complete item/persistence and bounded ground reservation
  before the irreversible block edit. Empty identity pools retain owned drops.
- Ground activation is exclusive with native durable ownership and generic
  entity writers. Each live payload belongs to the original canonical
  EntityStore slot; up to64 additional canonical8x8 regional stores permit the
  legacy one-shard world-cover application without weakening owner checks.
  Global admission caps live/pending/offered/restoring items at4096 and native
  reserved bytes at both the selected item budget and the selected<=4MiB limit.
- Trusted item metadata must encode before admission and fit the same reserved
  native charge. Thus aggregate publication metadata cannot exceed that global
  reservation. Adoption rechecks existing payloads. Refusal retains the entire
  drop/restore input; pending unloaded entries rotate without losing payloads.
- Pickup moves the actual payload through a single-use incarnation ticket.
  Until the exact FCIS remainder is returned, its native slot/byte credits stay
  reserved and capture refuses. Empty remainder means actual complete deposit;
  no timeout, disconnect or cloned inventory projection proves that deposit.
- Logical item counters advance once per resident tick for all admitted items.
  Physics/contact/publication use a fair256-item phase, an outbox256/2MiB cap,
  and at most32 offers/responses. Full publication capacity retains pending
  work; destruction can be delayed until a publication credit exists. Coarse
  full-cube physics, no merge/activation/plugin events, partial-return delay10,
  ordinary unenchanted mining and deterministic FerrumC loot RNG are explicit
  limits, not claimed Paper differential acceptance.
- Regional migration preflights canonical destination payload/residency/slot
  before source removal, preserving original UUID, wire ID and owned stack.
  Snapshot clones receive no second active consumer. Exact native record capture
  is readonly and neither acknowledges persistence nor clears dirty data. Legacy
  graceful sidecar save/restore is not native DUR activation or crash atomicity.

## Selected legacy player vitals consumer

- enable_survival is default-off, once-only and exclusive with native player
  ownership. Fixed-size health/hunger/timers live on actual PlayerState; native
  inventory admissions have no legacy vitals producer. Snapshot copies lose tick
  authority. All restore/food/damage/fall/respawn inputs mutate only at a boundary.
- Health and hunger tick once per present actor; all9 saved fields are queried
  from that actual owner. Session sends mandatory UpdateHealth from its snapshot.
  Effective damage and one death edge follow committed health, never HUD inputs.
  Dead players cannot mine/place/pick up/consume/attack; respawn requires dead
  state and resets vitals and actual position. Death inventory/location policy,
  CombatDeath/Respawn sequence, status effects and Java events remain App contracts.
- FoodConsumptionResolved carries a positive monotonically increasing per-player
  App token. A repeated token cannot apply food again. player_can_eat is a probe,
  not inventory authority: the connection must freeze its complete exact prepared
  inventory mutation until the matching owner acknowledgement and must commit
  only accepted results. No food inventory atomicity is claimed without that App
  pin/commit contract. Legacy UUID inputs retain existing Session lifecycle scope;
  this does not mint native admission, receipt or durability authority.
- Selected Survival applies earlier coalesced move prefixes before nonmovement
  actions, so reach/contact/death inspect actual preceding positions. Default-off
  replay ordering stays unchanged. Fall only lands on coarse full-cube support,
  water clears distance, and teleports>64blocks reset it. Sprint exhaustion uses
  actual ground/sprint flags with real coarse support. Partial support geometry,
  airborne anti-cheat, swimming/jump exhaustion and general environmental damage
  are explicit remaining work. PVP uses coarse3block contact and10tick attack
  interval; weapon-strength curves, obstruction, knockback and armor wear are
  separate consumers, not claimed Paper combat parity.
- Paper full FoodData,Player hurtServer/canEat/causeFallDamage,ServerPlayer
  tickRegeneration,and LivingEntity hurtServer/handleEntityDamage/actuallyHurt/
  calculateFallDamage were source-reviewed at the pinned build60 commit. Ordinary
  armor/cooldown/food formulas are checked deterministically; no Java events or
  gameplay differential acceptance are inferred from source inspection.

## Selected ordinary-water and furnace consumers

- Water and furnace selection are once-only legacy owners, exclusive with native
  player durability. Clones carry no tick authority; dimension/world checks run
  before inbox drain. Water advances by logical ticks only; at most64due cells
  and6writes per admitted plan, five-tick delay, no force loads. Complete changed
  postimage scheduling and ground native/metadata/region reservation precede any
  flow write. Refusal keeps the entire preimage and retains due work for retry.
- Bucket tokens are positive and increasing, with exact source-only filling,
  resident/mode/living/reach checks and full queue admission before one world
  mutation. Only an accepted BucketUseResolved permits App's pinned full player
  inventory commit. App owns actual UseItem ray geometry/eye pose and complete
  bucket inventory postimage; this position API alone is not real-client use.
- Furnace World data remains the sole owner of three slots and four timers.
  Registered resident ordinary furnaces are capped at1024; discovery admits64
  block entities across at most4columns per tick (each column already has a
  fixed4096entity ceiling), while all registered furnaces tick exactly once.
  Unload stops timers and frees registration; imported data is rediscovered.
  Placement reserves registry and native block-entity capacity before writing.
- A furnace tick admits its complete native postimage and lit-state transition
  before consuming fuel/input. Lit changes use MutationCause::Furnace through
  the existing journal/persist/block publication funnel, preserving block data.
  Every changed timer/slot body marks persist dirty, even without a block change.
  Mining reserves all three content stacks plus normal block drop before removal.
- Checked furnace exchange compares exact FCIS slot bytes inside the owner,
  excluding ticking timers. Stale returns actual data and unchanged player/cursor.
  All furnace/player/cursor candidates are staged; full16KiB native codec admission
  precedes one furnace commit. Input representation replacement resets cooking;
  count-only changes preserve progress. App must freeze/preflight its complete
  56-slot PlayerData and commit only the returned postimage after owner success.
- Original full FlowingFluid/WaterFluid/BucketItem/ClipContext and furnace
  AbstractFurnaceBlockEntity/AbstractFurnaceMenu/AbstractFurnaceBlock plus owning
  source patches were read at Paper build60. Ordinary pure rules and owner tests
  do not claim Java event parity, furnace recipe XP, blast/smoker/hopper mechanics,
  full Paper fluid geometry, lava/ultrawarm policy or crash-atomic legacy commits.

## Selected adult mob gameplay consumer

- enable_mobs is default-off, once-only legacy ownership, exclusive with native
  durable player admission and generic entity writers. Snapshot clones retain no
  live health/controller authority and refuse before inbox drain. Actual canonical
  base-only EntityStores own every UUID/wire/type/transform; the pure controller
  owns only health/brains and never a second transform. Global64 mobs,16pending
  attempts,64canonical stores/player profiles and256fixed-size events bound work.
- Separate base-only stores never consume ground-item pending/offered slot or byte
  credits. UUID/wire uniqueness is checked against ground active/queued restore/
  offered/identity pools, player profiles and mob pending/live registrations in
  both admission directions. Driver uses the same global nonreusing Session IDs.
- Actual movement updates EntityTickContext; cross8x8 migration preflights genuine
  destination identity, residency and slot reservation before removing source.
  Same UUID/wire/type/health survive migration. No force-load or raw transform
  escape exists. Unloaded actors remain registered but stop active walking/attack.
- App supplies verified resident sky/block/raw light and actual wire/armor player
  profiles. Natural spawn owner rechecks resident dry collision facts, headroom,
  floor/light/difficulty, genuine24..128player distance, sharedspawn24 exclusion,
  border, population and existing mob obstruction. Any intervening funnel block
  edit after enqueue invalidates the admitted light basis and refuses the spawn.
  App remains responsible for current relighter residency/generation validation,
  native biome/category selection and its bounded natural attempt cadence.
- Pure bounded ground walking/passive wander/zombie chase and20tick melee run
  every logical owner tick. Movement publication pressure may refuse intentions;
  positive hostile damage uses current trusted armor plus actual player vitals.
  Dead/Creative/Spectator/disconnected players are excluded. No guessed player
  wire IDs or lighting values are minted by this owner.
- Tracked mob hit checks positive increasing exact App token, actual player
  strength/reach/mode/residency and controller hurt immunity/armor. Every death
  stack and regional/native/metadata credit plus final health/remove publication
  is reserved before zero health commits. Full ground/outbox retains live health
  and original loot RNG. Successful owned receipt moves actual drops into ground
  admission once; death removes genuine canonical registration once. Root applies
  exact weapon wear and credited XP only from accepted MobAttackResolved.
- Source-reviewed original NaturalSpawner, Monster/Animal spawn rules,
  MeleeAttackGoal/ZombieAttackGoal and author-pinned JVM attributes/collision/loot
  evidence support this explicit adult subset. No full goal/pathfinding/travel,
  breeding/shearing/eggs/riding/equipment/knockback/daylight burn/despawn/Javaevents
  or complete Paper gameplay differential is claimed. Native capture/restore
  remains separate from actual sidecar durability, described below.

- Legacy death-drop admission takes one move-owned batch of at most 56 stacks.
  Complete slot/native/metadata/publication and aggregate regional-claim checks
  finish before any stack or once-per-life flag changes. Refusal returns all
  original inputs. Specialized player-death admission requires the real dead
  actor's exact position, and permits one accepted batch (including empty) per
  life; actual respawn resets it. This flag is not a durable receipt. App pins
  the original complete inventory until successful admission and owns clearing
  its player/cursor/table data; no cross-store crash atomicity is claimed.
- Withdrawing a trusted mob player profile removes that player from the actual
  AI target list. Missing armor projections never receive default armor or an
  AI hit. App uses withdrawal while armor receipts/effect capacity are reserved.

- Legacy mob capture emits the exact canonical owner base plus authoritative
  controller state into independent FCMB native schema1 (95 bytes per mob,
  global64/6080 native bytes). UUID/type/scope/chunk, finite canonical transform,
  positive bounded health, loot/brain RNG and remaining wander/attack/hurt
  durations are strict; malformed/schema/truncated/trailing records refuse
  without defaults or mutation. Runtime wire IDs/handles/admissions/leases do
  not persist. Pending proposals refuse the entire capture; events alone do not
  duplicate canonical authority. Capture clears no dirty/save ownership fact.
- Saved mob restores retain at most16 complete decoded proposals and fresh
  genuine wire IDs, preserving the original UUID, with global64 population
  admission. An unloaded or full-publication target retains the original queue
  member; no force-load occurs. Actual tick-context registration precedes fresh
  controller admission; a controller refusal removes only that new base slot.
  First real restore tick advances saved remaining logical durations once.
  App combines item and mob records in one existing identity-bound save/delete
  barrier; capture alone proves neither durable save nor crash atomicity.
- Withdrawing a mob profile disables actual AI targeting but retains its genuine
  player wire reservation until departure. Ordinary set restores targeting with
  fresh armor stats. Player UUID admission refuses identities reserved by live,
  queued-natural or queued-restored mobs; all ground/mob reservation checks also
  include queued-restored identities.

- Legacy pickup completion publishes accepted collect before either partial-stack
  metadata or final entity removal, retaining the prior wire stack and viewer
  identity for the client pickup animation. Admission and reserved byte/slot
  settlement finish first; refused remainder restoration publishes no collect.

- Legacy chest drag compares the entire exact FCIS beforeimage and stages the
  whole chest/player/cursor result under the existing exclusive owner. Current
  and candidate entities must fit the existing 16 KiB persistence cap before
  one replacement/dirty mark. Conflict/budget refusal publishes fresh chest
  with unchanged player slots/cursor and no partial slot or dirty mutation.
  Absent/dead/spectator actors, reach/residency and native ownership refuse.
  App must admit its complete hidden player/table save budget before calling.

- Legacy PlayerLeave publishes the complete actual nine-field vitals before
  actor removal, including silent same-boundary exhaustion changes. This is a
  save observation, never another mutable player owner or a client health packet.
- Mining environment refresh preserves exact held-tool token and elapsed work.
  With water enabled, progress reads current resident standing-eye fluid states;
  it never force-loads. Zero hardness remains instant under air/water penalties.
  Plant chance uses the existing owner RNG and commits it only with the full
  admitted block/drop postimage. Paired plants validate and preflight both cells.
- Explicit water enable seeds already-resident columns through a resumable owner
  sweep. App marks each newly resident acquisition with `wake_water_chunk`, also
  waking resident cardinal borders. Discovery changes no tickets and never loads
  columns. Snapshot clones retain no live discovery/tick authority.
- Water discovery keeps at most4096 scalar column cursors and one sweep/resweep
  state. Ordered resident-map range lookups admit at most4sweep keys/tick. Wake
  overflow retains work rather than failing a successful acquisition. Each tick
  probes at most8192cells/empty-section skips, with at most5additional reads per
  source; round-robin slices are at most256cells. Queue pressure preserves the
  exact unadmitted cell and leaves up to42credits for complete flow admission.
  Enclosed sources stay asleep; flowing cells re-evaluate their resident basis.
- Eye-water uses original f32 amount/9 widened to double, fullheight1 only with
  resident water above, and strict surface>eye. Pose/boat policy, native fluid
  persistence, lava, entity travel/forces, Java events and exact imported due
  times remain separate ports.
