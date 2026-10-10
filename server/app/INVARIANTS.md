# Invariants: ferrumc-app

> Rules that must hold for all code in this crate. Violating these is a bug.

## General

- No `unwrap()` or `expect()` outside `#[cfg(test)]`.
- No unbounded channels or allocations from untrusted input.
- All public items have rustdoc.
- Error types classify the failure mode.

## Crate-Specific

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
- A chest click may exchange items only after both active window and state IDs
  match. The driver also compares that viewer's bounded chest snapshot atomically
  before a chest-slot exchange using exact lossless item encodings, including
  NBT float bits; conflicting views return the unchanged cursor and fresh
  contents. Resyncs include both slot sections and the cursor.
- A Close Container packet may close only the window whose exact ID it carries.
- Normal window-0 clicks derive changes exclusively from authoritative slots and
  cursor. Spectators cannot exchange items in either section of an open chest
  or in window 0. Crafting results remain read-only until recipes exist; armor
  insertion waits for equippable rules. Full-bag crafting input overflow remains
  in its input slot until ground-item handling exists.
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
- New player saves use binary schema 3: exactly 46 slots and a required carried
  item, each framed with the lossless bounded FCIS item codec. Typed/opaque
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
