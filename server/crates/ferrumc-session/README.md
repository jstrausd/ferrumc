# ferrumc-session

Bridge between network and simulation: typed bounded-delivery policies,
player/shard routing, presentation translation, and a lease-validated directory
that maps logical 8×8 `ShardId` regions—or a world-covering fallback—to runtime
endpoints. Native coverage (NET-04.2e1) adds sealed session keys, the checked
shared wire allocator, native join reservation/admission, a bounded native
session lane with movement coalescing, and closure-scoped endpoint
revalidation.

## Invariants

See `INVARIANTS.md` in this directory.
