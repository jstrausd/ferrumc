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
