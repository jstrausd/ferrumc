# Invariants: ferrumc-app

> Rules that must hold for all code in this crate. Violating these is a bug.

## General

- No `unwrap()` or `expect()` outside `#[cfg(test)]`.
- No unbounded channels or allocations from untrusted input.
- All public items have rustdoc.
- Error types classify the failure mode.

## Crate-Specific

- Production startup explicitly uses storage format4. One blocking opening task
  constructs the shared backend; persisted generation identity is reconciled
  and confirmed before Anvil import, spawn, player loading and client admission.
  An omitted seed or preset keeps the stored identity; a fresh omission explicitly chooses
  FerrumC-flat-v1/Overworld/1.21.8/seed0. Explicit mismatch and malformed metadata
  refuse startup without any world-bearing record change.
- An App-opaque PersistenceTarget couples its private exact store Arc with the
  immutable GenerationBinding and opaque verified token. Tick capture uses no
  database read and starts only after bounded queue reservation; capture errors
  retain dirty facts, journal and unassigned mutation IDs. Unload and final stop
  use the same target. Worker validates metadata before every write/retry and
  before empty durability acknowledgments; foreign overlay scope is refused.
- Natural preview startup uses a bounded one-worker executor outside Tokio workers.
  Streaming prepares immutable raw baselines with at most two blocking jobs,
  64 queued targets per viewer and 1024 viewers. Visible demand takes precedence
  over outer speculation across viewers, with round-robin within each class;
  up to two already admitted blocking jobs finish without preemption. Shared
  teardown cancels priorities even after failed join; completed late baselines
  are shape-validated and discarded when no active viewer still requests them,
  and own no tickets. At most 64 baselines and 8 MiB
  conservatively charged storage remain cached; each result is at most 512 KiB.
  Clean generated block entities (including complete cabin loot) are allowed;
  their real block state and lossless native codec must validate before stale
  output is discarded or retained. Persist-dirty, persist-edited and lit
  gameplay chunks are refused. Charge includes triple the palette-preserving
  section encoding, fixed Chunk/allocator allowance, biomes, conservative
  block-entity BTree nodes and every decoded item/component/NBT/string allocation.
  The retained clone removes excess producer Vec/String capacity without changing
  payload contents. Worker scratch and resident gameplay chunks are additional; these
  are allocation charges, not an RSS guarantee. At most 128 refused columns are
  memoized. Successful owner admission releases its exact raw cache copy and
  removes only those resident columns from viewer preparation queues. Late
  completions for resident columns do not repopulate the warm cache. Shutdown
  waits admitted blocking producers without starting more.
  Speculative jobs reserve one maximum512-KiB result per admitted/new job against
  the existing cache bounds; a full warm cache pauses speculation instead of
  repeatedly evicting and regenerating unused outer columns. Visible demand may
  still replace old baselines within the unchanged LRU bounds.
  Each viewer retains at most64 current intent positions separately from its
  consumed work queues. Plan replacement or departure releases cached baselines
  that no active viewer still requests, so stale saturated speculation cannot
  prevent a changed direction from preparing new columns. Already admitted
  blocking jobs remain bounded and finish without preemption.
  Cold targets acquire no tickets. Real admission uses the ordinary Sim gate,
  rereading exact metadata, full snapshots and overlays after preparation.
  Ticket ownership and outbound delivery are separate sets: a tail-dropped
  chunk or block entity retries a fresh resident snapshot without taking another
  ticket. Unload retains one extra ring to avoid boundary churn; desired chunks
  keep the current column first, then the next two forward columns and their
  one-column lateral margins within the actual visible view. Remaining columns
  are nearest-first, with movement breaking equal-distance ties. With no known
  heading, only Chebyshev ring two uses Manhattan distance before coordinates
  so its four cardinal columns are prepared before its corners. Every other
  stationary ring keeps the original contiguous coordinate order for terrain
  halo locality. Moving corridor ordering remains unchanged, and
  speculative targets extend two rings, biased toward the last actual horizontal
  movement until reversal or teleport. Each pump scans at most64 missing visible
  candidates so cold near columns do not hide ready visible columns behind them;
  the driver independently caps real acquisitions at16 in both natural and flat
  paths. Snapshot retries subtract from the same16-column connection budget; block-entity
  packets for those columns are additional.
  Unload batches remain bounded to16.
  Client render distance can lower the server cap; negative or absent sentinels
  retain it. Configuration and Play settings use the same bounded schema; the
  Play id0x0d body must be exhausted and repeated settings do not renew pre-Play
  phase deadlines. This is a bounded FerrumC policy, not full Paper loader parity.
  Flat spawn remains configured; natural spawn selects a dry support column
  with two clear blocks within the bounded resident spawn square, and `/spawn`
  uses that same position. Startup reports the chosen revision and remaining
  placed-feature/ore/tree/structure gaps; density-generated ore veins are included.
  Natural preview refuses Anvil imports before declaring metadata or importing
  chunks, including when the natural preset comes from stored metadata; imported
  full snapshots do not preserve the preview biome palette payload.
- JoinGame's biome hash uses the stored numeric seed and original Minecraft
  SHA256/hashLong/asLong byte order, covered by actual original-JAR goldens.
- Legacy format3 requires the explicit offline migrate-generation operation,
  dispatched before config bootstrap or socket binding. Prepare/backup/migrate
  run on a blocking worker. New backups are exclusive; reuse requires the
  remembered physical SHA256 and full snapshot validation. No automatic seed
  inference, repair, startup migration or backup overwrite is permitted.

- Built-in demo plugins default to disabled, including when the TOML toggle is
  omitted. Explicit `builtin_plugins=true` enables their sample behavior
  (glass-to-tinted-glass replacement, bedrock denial, greeting/chat filter).
  Native and Java plugin loading use their own configuration. Existing explicit
  opt-ins are retained; loading a config never silently rewrites it.

- Pre-Play phase progress has an absolute configured I/O window. Only the
  expected semantic phase transition renews it; raw bytes, ignored IDs and
  repeated settings do not. Partial frames have a separate first-byte window
  capped at 10 seconds and never renew on drips. Pipelined tails retain their
  original read timestamp; pre-Play output and shutdown observe these deadlines.

- Established Play retains at most one decoded frame on token exhaustion and
  stops socket reads until real configured-rate refill. The first exhaustion
  starts one recovery episode no longer than burst/rate or two seconds; only
  actual half-burst refill resets it. Continued starvation after the episode is
  a typed BudgetExceeded disconnect. Initial login-handoff drain remains strict.
  The quota wake participates in normal bounded round-robin work; shutdown and
  absolute liveness deadlines keep priority. Held bodies cause no packet effects
  or deadline renewal. Burst/rate ceilings remain unchanged. This finite FerrumC
  policy can still reject larger accumulated backlogs and is not Paper parity.
- Play has absolute inbound-progress and Keep Alive response deadlines. Chunk,
  output, and Keep Alive timer wakeups never extend them.
- Each Play connection has at most one unanswered Keep Alive. Its exact ID must
  be echoed once before the response deadline; unsolicited, mismatched, replayed,
  and late echoes terminate the connection through the normal leave-save path.
- A Keep Alive's response clock starts after its successful socket flush. A
  shorter configured send interval never replaces an unanswered request.
- Ready inbound, outbound, timers, and operator graph changes rotate in bounded
  round-robin order below shutdown/deadline priority. Continuous revisions cannot
  starve echoes or chunk/output work.
- Each connection assigns every join, command, plugin or correction position
  sync a fresh teleport ID, wrapping from `i32::MAX-1` to zero, and retains only
  the newest pending acknowledgement. A stale ID is ignored; the current ID
  without a pending teleport is a fatal protocol violation. Pending teleports
  suppress client coordinates, movement callbacks and movement-driven chunk
  work; finite validated look can still be admitted. The authoritative server
  destination is persisted independently of the client's acknowledgement.
- Before each non-Keep-Alive input, the connection freezes at most the existing
  256-message outbound FIFO prefix and sends it one message per normal flush
  turn. Production cannot extend that prefix; its held input runs before a new
  prefix. At most 256 extra messages and one already frame-limited body are
  retained (variable byte sizes). Exact Keep Alive bodies at the input head can
  bypass this positional fence; all strict validation and deadlines still apply.
  Teardown mirrors retained/queued authoritative syncs in bounded FIFO order
  before leave-save. A teleport adds no separate resend/response timer: the
  existing absolute Play liveness and shutdown limits remain in force.
  The ACK gate protects connection-issued syncs, and the frame fence orders
  outputs already queued when captured. External teleports queued concurrently
  after that snapshot require a separate producer/consumer epoch-admission
  contract; this App snapshot does not establish global atomic tick ordering.
- Cancellation-safe chunk/socket awaits honor the active absolute deadline.
  Stateful inventory packet handlers and container transactions are never forcibly canceled;
  transactions complete before their updated inventory is persisted.
- A canceled chunk-stream send retains unload bookkeeping until admission; a
  canceled reply releases only actually acquired tickets behind a durable flush.
- Join starts with an empty acquired-ticket set. The actual `StreamChunks` reply
  transfers ticket ownership before packet encoding/flush; failed setup before
  that reply releases no guessed spawn tickets, and an unconfirmed admitted
  handoff quarantines the player UUID.
- A chest click requires the active window ID. A valid older client StateId
  does not deny a predicted pipelined action: the action uses the current server
  slots and cursor, then receives complete authoritative content. Client
  changed-slot/cursor predictions never grant or replace items. The driver also compares that viewer's bounded chest snapshot atomically
  before a chest-slot exchange using exact lossless item encodings, including
  NBT float bits; conflicting views return the unchanged cursor and fresh
  contents. Resyncs include both slot sections and the cursor.
- A Close Container packet may close only the window whose exact ID it carries.
- Normal window-0 clicks derive changes exclusively from authoritative slots and
  cursor. Spectators cannot exchange items in either section of an open chest
  or in window 0. Normal 2×2/3×3 crafting results are derived from the pinned
  shaped/shapeless recipe catalog and consume one complete craft only after
  output and all remainders can fit. Armor insertion waits for equippable rules.
  Full-bag input overflow remains in its input slot. Nine crafting-table inputs
  use lossless schema-4 frames; schema-3 migration supplies canonical empty
  table inputs and never rewrites a loaded source record.
- Chest shift transfers and number-key swaps return complete chest/player
  postimages from the checked shard transaction. Component capacities and exact
  ordered stack representations govern merges; stale snapshots mutate neither
  side. App adopts both outputs before the next save and carries no cursor in
  these transfers. Fresh players receive an empty Survival inventory.
- The player inventory is the sole owner of the carried stack across every
  screen. Closing or disconnecting attempts a deposit and retains its overflow;
  window 0 and chest resyncs always encode that same stack.
- Normal shutdown drains every connection's save, ticket release and Leave
  acknowledgment while the driver remains alive, then stops simulation and
  awaits the final storage drain. Terminal task failures remain observable.
- Operator identity uses exact case-sensitive offline names/UUIDs. Configuration
  `ops` are seeds; strict schema-1 `world_dir/operators.json` overrides take
  precedence, including explicit revocations. This is a FerrumC record, not
  Vanilla `ops.json`; malformed records refuse startup without rewriting.
- Persistent native-only startup registers single-target `/op` and `/deop`.
  Both level 3 and current operator identity are required, so default level 4
  never permits self-promotion. Offline targets are supported. Explicit default
  levels and separately configured bypass grants survive deop.
- Operator writes are serialized and recheck the issuer after earlier commits.
  There are at most 1024 seeds, 1024 overrides and 128 KiB per record. Blocking
  writes run off Tokio workers: exclusive staging, file fsync, atomic rename,
  directory fsync precede live publication, graph revisions and success feedback.
  Pre-rename failure retains old state. Uncertain post-rename durability denies
  all operator identities, stops admission/mutations, requests fatal shutdown,
  and remains an error after drain. Player/world writes are separate transactions.
- Operator command callers observe Play deadlines and stop while waiting.
  Once accepted under the mutex, an owned commit finishes despite cancellation.
  Stop closes admission before signalling connections, then drains the accepted
  commit; previously queued mutations cannot commit during that drain.
- The trusted local API and bounded stdin console (16 lines, 256 bytes per line)
  provide recovery after the last operator self-revokes. Console EOF disables
  input; signal/internal stop remain observed while commands await persistence.
- Java-enabled mutation stays disabled until durable Rust changes and every
  affected retained Java permission snapshot have an acknowledged fail-closed
  refresh contract. This native-only step refreshes Commands but sends no client
  EntityEvent permission-level status and does not recalculate Bukkit permissions
  or kick players on whitelist changes. Offline login retains username trust.
- Production overlay capture uses the strict fallible codec, at most four
  records per request. All selected records validate before dirty masks, the
  mutation journal or assigned mutation IDs transfer. A full eight-slot storage
  queue defers tick capture; release barriers acknowledge every batch before any
  ticket release, and shutdown drains all remaining batches.
- Storage commits on an 8 MiB encoded-overlay threshold as well as existing
  count/time limits; accounting decreases only after a successful prefix save.
  Record ceilings bound each request to 12 MiB encoded overlays. Pending data
  can overshoot its threshold by one request; journals and in-memory object
  overhead are additional. These are encoded limits, not an RSS guarantee.
- New player saves use binary schema 4: exactly 46 slots, a required carried
  item, and nine crafting-table inputs, each framed with the lossless bounded
  FCIS item codec. Schema 3 explicitly migrates with empty table inputs. Typed/opaque
  components, removals, NBT ordering and floating-point bits survive saves.
  The aggregate payload is at most 1 MiB, checked before each append; at most
  one temporary item buffer is retained during encoding. Malformed data refuses
  admission. JSON schemas 1 and 2 explicitly migrate with their historical slot
  normalization; schema 2 retains its strict cursor/layout/known-field rules.
  Loading never rewrites records and cannot recover components omitted by old
  saves. Player/world crash-atomic transactions remain unimplemented.
- A bounded server-owned UUID lease is acquired before player loading and held
  through durable leave-save and acknowledged simulation Leave. Every error
  after accepted Join runs shared teardown. Save/encode/Leave-ack failure
  quarantines that UUID until this server instance drops; quarantine counts
  against the configured connection ceiling. Restart clears admission state,
  but does not repair a failed durable write.
- An inaccessible chest refuses exchanges and retains the last snapshot/cursor;
  server-driven closure of a vanished or unreachable chest is not yet modelled.
- Java compatibility is optional. Native-only startup neither launches nor
  requires a JVM. Both plugin-directory and compatibility-JAR paths must be
  configured together before a child can start.
- One bounded actor owns the Java child, stdin and stdout. At most 32 requests
  wait; full admission returns an explicit error. Every frame is UTF-8 JSON with
  a four-byte big-endian length in `1..=65536`, checked before allocation.
- A Java RPC has one absolute deadline covering queue time, writing and reading.
  Expired/cancelled queued work is skipped. Cancellation, deadline expiry or
  corrupt active framing/IDs kills and reaps the child before any later request
  can consume a delayed response. Shutdown waiting also observes cancellation.
- Java metadata is frozen before connections are accepted. There are at most
  64 plugins, 256 commands, 256 aliases in total and 16 aliases per command.
  Plain routes and their standard plugin namespaces yield at most 1024 routes;
  builtin/native/Java collisions are checked atomically before registration.
  Commands and aliases resolve case-insensitively, matching the Java command map.
- Java responses have at most 128 messages with 4096 UTF-8 bytes each; error
  text is at most 2048 bytes. A target is either the current request's exact
  canonical UUID or the console. No connection, online roster or broadcast
  capability is exposed to Java. Departed-player quit messages are discarded.
- Rust supplies the canonical UUID/name and live operator membership.
  The configured default command level never implies Java operator status.
  The first permission snapshot includes only allowed command-declared nodes;
  Bukkit defaults, children, wildcards and attachments are not implemented.
- Java callbacks run through the actor outside simulation/storage ownership.
  Unimplemented world/scheduler APIs report a concrete callback error. A remote
  callback error preserves framing; a protocol error stops the actor.
- Java argument nodes declare `minecraft:ask_server`. Completion uses Rust
  command permissions and the current sender, preserves Bukkit's literal-space
  empty arguments and original alias casing, and reports UTF-16 ranges. Only
  completion responses contain up to 64 suggestions of 256 UTF-8 bytes each.
  Stateless completion awaits observe the active Play deadline and shutdown;
  cancelling an active RPC may force process termination and skip disable hooks.
- Startup rejection awaits process cleanup before returning and accepts no
  clients. Normal shutdown awaits connection quit-request outcomes before
  attempting one plugin shutdown request, then awaits child exit. A healthy
  adapter delivers these hooks; forced termination, including cancellation of
  an active completion during stop, cannot guarantee Java quit/disable hooks.
  The process boundary is not an OS permission sandbox.

- Player-placement after-hooks wait for the dedicated tracked simulation result,
  including the final primary position and state. Generic plugin intents cannot
  satisfy that completion. Refusals emit no success hook. The live final shape
  gate includes actor/other nonspectator bodies and all door/bed cells before
  mutation. Interaction target resolution admits only the explicitly supported
  short-grass replaceability subset; the tick revalidates mutation authority.
- The explicit natural-grass preview binds algorithm2 revision2; omitted startup
  declarations preserve its stored identity. Its isolated original plains/forest
  short-grass features are not full FEATURES. Spawn skips grass to a real dry
  support and both preview revisions reject Anvil imports before metadata writes.

- The explicit natural-tree preview binds algorithm2 revision3; omissions keep
  its stored seed/revision, and explicit older-preset changes refuse before world
  writes. Natural imports remain forbidden. Spawn scans at most384 cells per
  candidate through air/short-grass/oak/birch canopy/trunks to first real terrain,
  requiring two passable body cells and centering the body inside that column.
  Tree blocks never support spawn, fluids and
  unknown obstructions stop descent. Revisions1/2 retain their previous policy.
  The oak/birch subset is incomplete FEATURES and does not imply Survival.

- Initial joins center the client on the actual fresh/restored destination.
  The destination column must be acquired, successfully queued at mandatory
  initial-join admission and flushed before the first position sync. State
  priority cannot overtake that separate World flush. Cold generation retries
  share an absolute join deadline and observe shutdown; actual returned tickets
  are transferred before socket work and canceled late replies release at driver.
- Minecraft and dashboard default to IPv4 wildcard addresses. The explicit
  `--bind-all-interfaces` startup flag applies after bind/port overrides, preserves
  each listener port and dashboard enablement, and is used by the preview launcher.

- Selected legacy Survival enables water with4096due cells and ordinary furnaces
  with256registered owners. Bucket preparation freezes the complete inventory
  until an exact token/sequence/fill owner result; only positive state commits.
  Rays read resident columns only. Furnace GUI uses the shared window allocator,
  FCIS slot comparison inside Sim and a complete player-save preflight before
  exchange. Timer-only updates never invalidate item revisions. Container chunk
  descriptors expose only empty render tags, never inventory or furnace timers.
  Current descriptors encode present empty compounds (Vanilla serializes null).
- Stored mutation causes preserve tags0..3; ordinary water/furnace transitions
  use new tags4/5, so audit records retain their actual system origin.
- Delayed mining wear searches exact beforeimages in all56player-owned locations,
  including cursor/table. Missing/ambiguous beforeimages refuse. A new mining
  start with any unsettled witness cancels live work and rejects/ACKs the new
  prediction before creating another token. Only the processed terminal owner
  receipt retires the old witness; clearing the active selection does not.
  A deferred committed break retains its witness through later uncommitted
  releases while the complete food snapshot is pinned. This conservative
  receipt serialization is a FerrumC policy, not Paper tick/input equivalence.
  While any mining receipt is unsettled, attack, held drop/swap, inventory
  exchange, item use/placement and commands refuse before owner mutation or
  plugin dispatch. Live mining is canceled without retiring its beforeimage;
  inventory/screen and block predictions are healed. Finish/abort, selection,
  close, movement and liveness remain available. Separate reply/effect queues
  never imply that an earlier mining wear has been processed.
  Creative starts use token0 and retain no unused wear witness: the actual
  Creative owner branch charges no tool wear. An already unsettled Survival
  witness still blocks mode changes and subsequent starts until its receipt.

- Tracked combat retains exact hand/armor FCIS beforeimages. Victim admission
  reserves one effect channel slot and excludes overlapping PvP/AI armor use
  until the sole inventory writer applies the accepted wear receipt and returns
  a fresh profile. Ordinary refresh cannot release that reservation. Unsupported
  or oversized profiles refuse attacks; complete-save wear headroom is checked
  at capture and again at commit. Ordinary inventory growth must retain the
  complete projected-DAMAGE envelope described below; cross-store crash
  atomicity remains separate.
- Active mobs use only genuine resident computed UpdateLight channels and the
  owner entity identity allocator. Cached light is evicted on relevant edits.
  Natural preview attempts are bounded and stop without connected players.
  The adult sheep/cow/pig/chicken/zombie subset has simplified walking/combat;
  the spawn schedule is preview policy, not original natural-spawner RNG parity.
- Death carries the actual owner position into the serial sole-inventory
  handler. All real inventory/cursor/table drops are preflighted and admitted
  as one dead-owner batch before complete empty postimages are adopted. The
  virtual result cell never grants drops. Pending food settles before death
  clears the pinned inventory. Refused/unknown admission retains the original
  inventory; the once-per-life owner gate is not a durable crash receipt.

- Ordinary connection-owned inventory edits admit a complete schema-5 56-stack
  projected-DAMAGE envelope before publishing their postimage. Missing DAMAGE
  reserves its exact 13-byte FCIS frame; opaque DAMAGE reserves any growth when
  replaced by a four-byte typed scalar. Future component count, cumulative
  payload and stack-frame limits are checked as well as the unchanged 1 MiB
  durable ceiling. The envelope covers every slot, cursor and table input,
  independently of game mode, so delayed wear cannot lose headroom to ordinary
  inventory growth. Container previews use the same exact shared planners and
  are admitted before world-owner commands; committed receipts are then adopted
  without a fallible post-commit refusal. Restored records remain losslessly
  loadable; an image without future headroom cannot publish combat or an ordinary
  candidate that still lacks the envelope. These resource checks do not establish
  atomic player-health/world/inventory persistence or Paper parity.

- PlayerAction status3/status4 prepare whole/one selected-mainhand ground drops
  with exact FCIS beforeimages of all 46 slots, cursor and nine table inputs,
  mode and selected index. The complete projected-save postimage is admitted
  before the bounded driver request; the non-canceled serial connection awaits
  a genuine bool reply before shrinking the selected slot. False keeps the
  original inventory; a lost outcome or changed beforeimage is fatal uncertainty,
  never denial/retry. The driver checks the actual present/living/nonspectator
  actor and resident current position, then the existing sole ground owner
  reserves payload/native/publication/pending capacities. Thrower is the actual
  player UUID and pickup delay is 40. Ground motion remains feet-origin with
  initial velocity (0, 0.2, 0), rather than original eye/pitch/yaw throw physics.
  Inventory resync, mining-tool notification, equipment and outer combat-profile
  refresh remain serial. GUI/cursor drops, Paper item-drop events/cancellation,
  stats, map tracking, same-tick drop-rate policy and crash atomicity are outside
  this held-mainhand slice.

- The no-argument launcher uses the native natural playable revision5 and a
  separate `local-play` save, with `RUST_LOG=debug` unless explicitly overridden.
  World metadata remains authoritative: existing generator revisions are never
  silently reinterpreted. The runtime reports the selected implemented subset;
  additional native structures do not establish full Vanilla world-generation
  or original structure-placement parity.

- Ordinary chest QUICK_CRAFT collects at most 63 unique compatible slot indices
  without mutation during start/add phases. Finish admits the complete player
  save envelope and commits one complete FCIS-checked chest/player/cursor
  postimage at the exclusive owner. Invalid sequence, changed cursor, conflicting
  viewer or resource refusal leaves the inventory parts unchanged. Left/right
  distribution is supported; Creative middle-fill and Java drag events remain
  separate behavior.

- Ordinary window-0 and crafting-table QUICK_CRAFT also retain a bounded drag
  selection tied to the active screen. Start/add mutate no items and send no
  correction; finish forecasts the complete 56-stack save and future-DAMAGE
  budget before staging component replicas. Results and armor cannot receive
  drag items. The single local stage derives recipe output and publishes all
  inputs/storage/cursor together; actual output clicks consume ingredients.
  Invalid phases, changed cursor or screen replacement cancel the selection.
- Ground pickup merges exact existing stacks in selected-hotbar, offhand and
  raw-inventory order, then fills empty hotbar indices 0..8 before the main grid.
  Cursor-return/close policy and chest shift-transfer ordering are unchanged.
  The player save-budget refusal returns the entire offered stack; pickup in an
  unrelated slot preserves the held tool's exact FCIS beforeimage.

- A ground pickup invalidates active mining only when the exact selected FCIS
  stack changes. Unrelated deposits and whole pickup refusals retain the active
  token and terminal wear beforeimage; changed selection cancels work without
  retiring an unsettled committed receipt. This does not relax receipt ordering.
