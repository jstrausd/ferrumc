# Invariants: ferrumc-app

> Rules that must hold for all code in this crate. Violating these is a bug.

## General

- No `unwrap()` or `expect()` outside `#[cfg(test)]`.
- No unbounded channels or allocations from untrusted input.
- All public items have rustdoc.
- Error types classify the failure mode.

## Crate-Specific

- Play has absolute inbound-progress and Keep Alive response deadlines. Chunk,
  output, and Keep Alive timer wakeups never extend them.
- Each Play connection has at most one unanswered Keep Alive. Its exact ID must
  be echoed once before the response deadline; unsolicited, mismatched, replayed,
  and late echoes terminate the connection through the normal leave-save path.
- A Keep Alive's response clock starts after its successful socket flush. A
  shorter configured send interval never replaces an unanswered request.
- Ready inbound, outbound, and timer work rotates in bounded round-robin order
  below shutdown and deadline priority, so an outbound burst cannot starve echoes.
- Cancellation-safe chunk/socket awaits honor the active absolute deadline.
  Stateful packet handlers and container transactions are never forcibly canceled;
  transactions complete before their updated inventory is persisted.
- A canceled chunk-stream send retains unload bookkeeping until admission; a
  canceled reply releases only actually acquired tickets behind a durable flush.
- A chest click may exchange items only after both active window and state IDs
  match. The driver also compares that viewer's bounded chest snapshot atomically
  before a chest-slot exchange; conflicting views return the unchanged cursor
  and fresh contents. Resyncs include both slot sections and the cursor.
- A Close Container packet may close only the window whose exact ID it carries.
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
- Rust supplies the canonical UUID/name and configured operator membership.
  The configured default command level never implies Java operator status.
  The first permission snapshot includes only allowed command-declared nodes;
  Bukkit defaults, children, wildcards and attachments are not implemented.
- Java callbacks run through the actor outside simulation/storage ownership.
  Unimplemented world/scheduler APIs report a concrete callback error. A remote
  callback error preserves framing; a protocol error stops the actor.
- Startup rejection awaits process cleanup before returning and accepts no
  clients. Normal shutdown awaits every connection quit hook before one plugin
  shutdown request, then awaits child exit. Forced termination cannot guarantee
  Java disable hooks. The process boundary is not an OS permission sandbox.
