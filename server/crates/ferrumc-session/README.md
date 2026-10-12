# ferrumc-session

Bridge between network and simulation: typed bounded-delivery policies,
player/shard routing, presentation translation, and a lease-validated directory
that maps logical 8×8 `ShardId` regions—or a world-covering fallback—to runtime
endpoints. Native coverage (NET-04.2e1) adds sealed session keys, the checked
shared wire allocator, native join reservation/admission, a bounded native
session lane with movement coalescing, and closure-scoped endpoint
revalidation. NET-04.2e2 adds the native session lifecycle: one unified session
table, retention tokens and records, the bounded leave-save state machine,
retirement, lifecycle acknowledgement and release closures, G3 adoption,
tracking between native sessions and the genuine pose producer.

## Invariants

See `INVARIANTS.md` in this directory.
