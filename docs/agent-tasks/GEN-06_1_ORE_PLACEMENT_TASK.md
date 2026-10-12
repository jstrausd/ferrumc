# GEN-06.1 — Overworld ore placement preparation

Root work order, 12.10.2026. The operator prioritizes natural world generation.
GEN-05.11 carvers are under independent contract/oracle review; this parallel
preparation owns no World production files and must not edit their evidence.
It prepares the next connected GEN-06 step, ordinary placed ore features.
GEN-05.4's density-driven ore veins are already a separate material producer;
they do not implement `OreFeature` or placed features.

## Ownership and baseline

Read `AGENTS.md`, `CLAUDE.md`, `docs/PAPER_REFERENCE.md`, World invariants,
architecture and the accepted GEN-05.4 through GEN-05.10 contracts. Read the
delivered GEN-05.11 contract as **unaccepted preparation**, not runtime
authority. Main at assignment is `bfe580e3`; Root records the actual
work-order commit separately. Storage WIP and private Sim candidates are not
accepted baselines.

Write only a new `docs/agent-tasks/GEN-06_1_ORE_PLACEMENT.md` and
`.codex-tmp/gen06_1/**`. Production, other task documents and every historical
freeze remain readonly. No Git, new dependencies or further agents. A private
archive/probe must be explicitly identified, with its exact baseline and
overlay. Runtime authorization follows independent review and belongs to a
later single-crate World ticket.

## Original sources and scope

Use Minecraft Java 1.21.8 and Paper build 60 at commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Read and pin complete base/applied
sources and both patch layers for `ChunkGenerator.applyBiomeDecoration`,
decoration steps and feature indexing, `WorldgenRandom` decoration/feature
seeding, `PlacedFeature`, configured feature dispatch, placement context and
the placement modifiers actually used by all builtin Overworld ore entries.
Include `OreFeature`, `ScatteredOreFeature`, their configuration and target
rule tests, air-exposure/discard rules, height providers, biome filtering,
`WorldGenRegion` reads/writes and the consumed block tags. Discover the real
registry keys and ordering from pinned datapack resources; do not guess an
ore list. Distinguish ores from non-ore deposits and density-driven veins.

Determine the exact finite transitive modifier/rule/configuration closure.
Separate shared placement primitives from individual feature algorithms and
their later chunk driver if a single World runtime ticket would be too broad.
Report the real missing producer data, including tags, motion-blocking
heightmaps, biome generation settings, neighboring chunk stages, multichunk
publication and post-processing. Do not invent stub dependencies or silently
activate incomplete consumers.

## Deliverable and evidence

Produce a concrete German contract with:

- Actual original feature order, step/index/RNG stream, RNG draw chronology,
  geometry and rule-test behavior, block mutation and visible outcomes.
- Resource/configuration/modifier inventory and exact pins, builtin admission
  limits, typed errors and policy deviations. A complete builtin ore scope
  needs every actual Overworld ore configuration and its placement chain.
- A bounded ownership model for neighboring reads/writes, deterministic
  transaction/publication, caps and work accounting with explicit exhaustion
  behavior. No global shared world or mutation from networking/storage.
- A precise producer/consumer split and recommended next runtime work order.
- Reproducible genuine original-method evidence where feasible, including
  placement/height/rule/RNG boundaries and air exposure. Expected results must
  come from the original methods, never a rewritten Java/Python/Rust formula.
  Controlled scripted-draw or block-region probes must be labelled as such;
  they are not genuine generated chunks or whole-FEATURES acceptance.
- A plan for genuine original generated chunks at the correct predecessor
  stage, including preceding decorations, neighbor accesses, biome order and
  post-write snapshots. If a faithful runnable region cannot yet be produced,
  report the exact gap; do not fabricate a ServerLevel/Chunk status or claim
  finished whole-chunk differential coverage.
- Fixtures/validators with malformed cases, normal and Python `-O` guards,
  independent provenance checks, and N-1/N/N+1/allocation tests for the later
  runtime. Preserve Vanilla and Paper differences and exceptions explicitly.

Freeze the contract and the explicitly enumerated evidence with SHA-256 only
after the delivered claims match the actual execution. Send Root progress,
then exact manifest paths/hashes, real gaps, open choices and the smallest
implementable next ticket. This preparation does not accept GEN-06, natural
world activation, Survival, structures, lighting, Nether/End or performance.
