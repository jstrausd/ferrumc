# GEN-05.11 — overworld carvers work order

Root work order, 12.10.2026. Continues the connected World chain after the
accepted GEN-05.10 surface column driver
([runtime review](GEN-05_10_SURFACE_COLUMN_DRIVER_RUNTIME_REVIEW.md)). GEN-05.10
writes the selected surface blocks into a chunk as `SurfaceSystem.buildSurface`
does; this ticket covers the following CARVERS step for the overworld, as the
original `NoiseBasedChunkGenerator.applyCarvers` does. World remains the only
production owner.

The next World slot first produces a concrete contract and a genuine original
oracle; runtime follows their independent Root review. Preparation owns a new
`GEN-05_11_OVERWORLD_CARVERS.md` and `.codex-tmp/gen05_11/**` only. Production
stays read-only during that phase. A later runtime authorization owns only
`server/crates/ferrumc-world/**` and new runtime evidence and delivery. Root
owns Git, the board, integration and cross-crate consumers. No new
dependencies, other agents or edits to historical GEN-05.x artefacts.

Read the complete real Vanilla 1.21.8 and Paper build 60 implementations and
both patch layers for the carver step: `NoiseBasedChunkGenerator.applyCarvers`,
`CarvingContext`, `CarvingMask`, `WorldCarver` and the overworld carvers
actually configured in 1.21.8 (`cave`, `cave_extra_underground`, `canyon` and
whatever the data pack really lists), `CaveWorldCarver`, `CanyonWorldCarver`,
their configured carver JSON, the per-chunk random seeding over the 17×17
neighbourhood (`WorldgenRandom`, `setLargeFeatureSeed`), the aquifer interaction
during carving (`Aquifer` and the carving fluid picker), the replaceable block
tag, the surface fix-up for grass/mycelium under carved blocks, and the biome
lookup that selects carvers. Pin whole sources, the actual original classes,
JARs, libraries and JDK, the frozen GEN-05.10 oracle inputs and the accepted
FerrumC producers this ticket consumes (GEN-05.4 material and aquifer output,
GEN-05.5–05.8 biomes, GEN-05.9/05.10 surface).

The concrete contract must settle:

- The exact carver loop: neighbourhood iteration order, seeding per source
  chunk and carver index, which carvers each biome lists and in which order, and
  how a source chunk's carver reaches into the target chunk.
- The block replacement rule: which states may be carved, how the aquifer
  decides air, water or lava and barrier blocks at each position, the
  `CarvingMask` semantics and the grass/dirt fix-up, all against actual chunk
  contents after SURFACE.
- Which aquifer and noise state the carver reads and whether it shares or
  recreates the GEN-05.4 aquifer; no fabricated state.
- The chunk mutation contract: write order, how a refused chunk publishes no
  partial result, the interaction with heightmaps and the GEN-05.10 buffer.
- Exact retained and scratch capacities, work bounds per carver, per source
  chunk and per target chunk, N-1/N/N+1 admission tests and allocation
  behaviour with a positive control. No live TPS, RSS or full-server speedup
  claims.

The oracle must run the actual original carver step on genuine original chunks
produced by the original generator up to CARVERS, and capture the carver inputs
and the resulting block states and carving masks. Cover cave, extra underground
cave and canyon hits, aquifer water and lava, carving through surface and
under oceans, negative coordinates, chunk seams and carvers reaching from
neighbouring source chunks, in both Vanilla and Paper modes. Record which
original code produced every input; do not fabricate state. Retain both
originals' outcomes and exceptions. Check first whether Vanilla and Paper
differ before requiring tests that assume a difference.

Provide reproducible fresh-output runners, exact input and output manifests,
guard checks with normal and Python `-O` behaviour, and a compact mechanical
fixture with a validator, as in GEN-05.9 and GEN-05.10.

Carried test conditions from the GEN-05.10 runtime review (RR10-H1–H5), to be
added by the GEN-05.11 runtime in the World crate: the production height scan of
`build_chunk` at y 319, lava versus water in the eroded and frozen extensions,
literal retained-byte totals (10,816 and 406,592), allocation measurement of the
binding, material, session and `UnclassifiedState` refusals, and an A1 buffer
test that starts from a published buffer.

Out of scope: features, structures, lighting, natural-world persistence or
activation, Nether and End carvers, Survival and Java plugins.
