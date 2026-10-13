# Placement collision and natural streaming validation

Accepted starting point: `a4ce84d78a90a0a5cc69d6c1b682d00b4c38bc96`.
Minecraft Java 1.21.8, protocol 772; reference Paper build 60, commit
`29c8822d90899c89d2689338e81a98f690bcba12`.

## Result

Player placements validate final collision shapes against the actor and other
live nonspectator players before mutating chunks, block entities or journals.
FIFO movement prefixes, slab merges, both bed/door cells and player-attributed
exact placement are covered. Refusal corrects the original prediction and ACKs
its sequence. Generic administrative mutation APIs retain their explicit bypass.

The App waits for a dedicated simulation outcome before notifying placement
hooks. Generic exact plugin intents cannot satisfy another placement's result,
even with a reused client sequence. Successful merges carry the actual primary
position. Break hooks still use the earlier admission-only contract.

Natural streaming prepares raw baselines in two bounded background jobs instead
of waiting for generation in the tick loop. Per-viewer priorities are fair;
late results own no tickets. Actual admission rereads saved snapshots/overlays.
Held tickets and delivered columns are tracked separately, so tail-dropped
packets retry snapshots without duplicate ownership. One retention ring avoids
boundary churn; two speculative rings prepare ahead of travel. The raw cache
has a 64-column / 8 MiB charge ceiling, additional to worker scratch and live
chunks. All admitted producers are awaited, including after a worker panic.

## Independent client evidence

Release executable SHA256:
`6fdf2d7489770c7871f9f36caf1d63db7acc8465076ffa2389954908be42f20c`.
The existing binary was first shown to insert gold directly into the actor's
feet; the strengthened protocol test failed on that authoritative state.

The new release passes `tests/blackbox/natural-world.mjs`:

- Revision1, view radius6: all169 requested columns arrive and remain through
  two further world-time updates. First convergence25.123s; restart24.574s.
- Foot/head collision requests return their original air state and action ACK;
  creative inventory remains intact. A legal gold marker applies.
- Exploration crosses both positive and negative chunk coordinates, preserving
  decoded terrain and every quart biome across a graceful process restart.
- Optional grass revision2, view radius1: genuine grass is visible in seed0.
  Clicking grass at `(8,64,14)` replaces that cell with gold, leaves the adjacent
  `(8,65,14)` unchanged and survives restart. Untouched terrain hashes match.

These are local functional timings, not an equivalent-load Paper TPS/RSS
benchmark. Initial169-column generation still takes about25s, and untouched
terrain is regenerated after restart rather than stored as a complete disk
baseline. Edited overlays are durable; far chunks may unload from RAM.

## Library evidence and checks

Collision fixtures exercise446432 genuine JVM supported cases, covering27902
states. Forty-four offset bamboo/dripstone/moving-piston states remain outside
this static shape contract and fail closed. Pose changes, all living entities,
Paper visibility exceptions and Java BlockCanBuildEvent parity remain open.

The optional `ferrumc_natural_grass_preview` identity is algorithm2/revision2.
It preserves the original isolated plains/forest short-grass kernels and
matches512 JVM placed-feature cases, RNG draws and nine source projections.
The World release suite passes428 tests, with seven existing instrumentation
cases ignored. World, Sim, Session and Placement author checks and independent
App streaming/result reviews passed. Combined App/Config checks pass424 tests across43 suites; the existing Java
fixture integration remains ignored because it requires explicit JVM/JAR inputs.
Combined all-target Clippy with `-D warnings` and workspace formatting pass.

Existing revision1 saves remain revision1. An explicit incompatible preset
refuses startup; a fresh separate world is required for revision2. Spawn skips
short grass to a real dry support. Narrow replacement is revalidated at the tick.
This does not provide full vegetation, trees, placed ores, structures, mobs or
Survival. A separate configured-birch kernel has been prepared and compared to
210 original cases; it is not activated or part of this delivered runtime.

Legacy positive-placement fixtures were adjusted to avoid placing into a live
body. Rotation/MVP fixtures explicitly bound their irrelevant view load; the
original timeout guards were retained. Movement validation now expects five
unloads after its two-chunk shift because the retention ring keeps the other
five. Its hostile-input, router, residency and leave-save assertions remain.

See [local commands and preview limits](LOCAL_NATURAL_WORLD.md).
