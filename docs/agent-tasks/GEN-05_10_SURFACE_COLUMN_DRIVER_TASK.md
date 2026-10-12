# GEN-05.10 — overworld SurfaceSystem column driver work order

Root work order, 12.10.2026. Continues the connected World chain after the
accepted GEN-05.9 surface rules
([runtime review](GEN-05_9_SURFACE_RULES_RUNTIME_REVIEW.md)). GEN-05.9 evaluates
one block from an explicit per-block and per-column context; this ticket
produces that context for whole columns and writes the selected surface blocks
into a chunk, as the original `SurfaceSystem.buildSurface` does. World remains
the only production owner.

The next World slot first produces a concrete contract and a genuine original
oracle; runtime follows their independent Root review. Preparation owns a new
`GEN-05_10_SURFACE_COLUMN_DRIVER.md` and `.codex-tmp/gen05_10/**` only.
Production stays read-only during that phase. A later runtime authorization
owns only `server/crates/ferrumc-world/**` and new runtime evidence and
delivery. Root owns Git, the board, integration and cross-crate consumers. No
new dependencies, other agents or edits to historical GEN-05.x artefacts.

Read the complete real Vanilla 1.21.8 and Paper build 60 implementations and
both patch layers for `SurfaceSystem` (`buildSurface`, the per-column loop,
stone depth above and below, water height, surface level and the minimum
surface, the bandlands and iceberg extras, frozen ocean and eroded badlands
handling), the `WorldGenRegion` and `ChunkAccess` interaction during the
SURFACE step, heightmaps it reads, and the `NoiseBasedChunkGenerator` caller.
Pin whole sources, the actual original classes, JARs, libraries and JDK, the
frozen GEN-05.9 oracle inputs and the accepted FerrumC producers this ticket
consumes (GEN-05.4 material and aquifer output, GEN-05.5–05.8 biomes, GEN-05.9
surface rules).

The concrete contract must settle:

- The exact column loop: iteration order over the 16×16 columns and over Y,
  how stone depth above and below, water height and the surface level are
  derived from the actual chunk contents, when the minimum surface is computed,
  and which heightmaps are read at which point.
- The steep and hole inputs: which neighbouring heights are read, from which
  chunk state, and when (the original reads them lazily; GEN-05.9 accepts them
  as a borrowed view per call).
- The bandlands and iceberg extras and every other block write outside the
  rule tree, with their own original randoms.
- The chunk mutation contract: which blocks may be replaced, the write order,
  how a refused column or chunk publishes no partial result, and the interaction
  with existing block entities.
- Exact retained and scratch capacities, work bounds per column and per chunk,
  N-1/N/N+1 admission tests and allocation behaviour with a positive control.
  No live TPS, RSS or full-server speedup claims.

The oracle must run the actual original `buildSurface` on genuine original
chunks produced by the original generator up to the SURFACE step, and capture
the complete column context and the resulting block states. Cover all overworld
surface biomes including badlands variants, frozen oceans with icebergs,
beaches, rivers, deep and lukewarm oceans, mountain peaks, swamps and mangroves,
negative coordinates, chunk seams and both Vanilla and Paper modes. Record which
original code produced every column input; do not fabricate state. Retain both
originals' outcomes and exceptions.

Provide reproducible fresh-output runners, exact input and output manifests,
guard checks with normal and Python `-O` behaviour, and a compact mechanical
fixture with a validator, as in GEN-05.8 and GEN-05.9. Check first whether
Vanilla and Paper differ before requiring tests that assume a difference.

Carried test conditions from the GEN-05.9 runtime review: a test that combines a
wrong seed with wrong settings to pin the resolver mismatch order (RR-H1), and a
test that pins the laziness of the biome memo through `work_used` (RR-H2).

Out of scope: carvers, features, structures, lighting, natural-world
persistence or activation, Nether and End, Survival and Java plugins.
