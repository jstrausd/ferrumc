# ferrumc-net

Tokio TCP, frame codec, compression, encryption, connection lifecycle, packet budgets.

## Invariants

See `INVARIANTS.md` in this directory.

## Item entity delivery

`SetEntityData`, `SetEntityMotion`, and `TakeItemEntity` default to
State/Mandatory. Together with the existing Spawn/Remove classification, a
successfully admitted State FIFO preserves Spawn -> Metadata -> Motion and
Take -> Remove across frame-count and soft-byte batch splits. Metadata tails,
quantized i16 velocities, and the original Take count remain unchanged.

This is a conservative local transport policy: no outbound motion consumer
currently proves recovery from a lost impulse or stop. Mandatory motion can
increase State-queue backpressure; no performance gain is established here.

`PlayWriter` does not own criticality. A full per-priority packet queue returns
`EnqueueOutcome::Dropped`, records the drop, and retains its prior FIFO. The
caller carries criticality from the outbound channel and must disconnect on a
mandatory refusal, including at State priority. `is_drop_fatal` identifies
only Critical priority and cannot replace that check.

Admission is per packet, with no bundle or reservation. Explicit overrides can
break dependent ordering: Critical metadata precedes State spawn, and State
remove precedes World pickup. A future viewer consumer must bind a common lane
and handle both channel and writer refusal. The writer tests prove encoded
queue behavior, not App disconnect, live item physics, Paper bundle atomicity,
plugin events, or a playable Survival server.
