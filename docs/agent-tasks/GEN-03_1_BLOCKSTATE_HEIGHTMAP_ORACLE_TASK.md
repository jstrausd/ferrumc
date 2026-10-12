# GEN-03.1 — Original block-state predicates for natural heightmaps

Root preparation assignment, 12 October 2026. Natural world generation is the
operator's priority. GEN-05.11 owns the exclusive World runtime writer;
GEN-06.3 prepares original Ore execution. This independent preparation closes
the missing classification reference needed by future natural heightmaps and
Ore's OCEAN_FLOOR_WG gate. It authorizes no production source changes.

## Scope and accepted baseline

Read AGENTS.md, CLAUDE.md, PAPER_REFERENCE.md, Registry and World INVARIANTS,
architecture/overview, relevant ADRs, GEN-06.1 revision2 and its review,
GEN-06.2/GEN-06.3 workorders, and existing Registry state lookup/generation plus
World heightmap implementations. Read these from the accepted archive. Shared
World is concurrent unaccepted Carver WIP and must not be built or copied.

Readonly genuine accepted-main baseline:
`.codex-tmp/gen05_11-runtime-root-base-storage47e/archive`, main
`47eadc4c14813c201184b552a87ca7b9c6f97db4`, 1490 files including 324 accepted
World and 93 Storage files. Adjacent `base.freeze.json` SHA256:
`58ba13d6418133bb49a154682ba1d82533f63a5e6184c6c2a8b07c6c15a47a75`.
Verify all archive bytes before and after. Accepted World remains a912b1c2.

Write only new `docs/agent-tasks/GEN-03_1_BLOCKSTATE_HEIGHTMAP_ORACLE.md` and
`.codex-tmp/gen03_1-preparation/**`. Original references, all other sources/docs,
historical fixtures/freezes, dependencies and lockfiles are readonly. No Git,
agents, shared builds or runtime edits. Private probes belong in your area.

## Genuine original census and behavior

Pin Minecraft Java1.21.8/protocol772 and Paper Build60/commit29c8822d, the
existing exact Vanilla/Paper inner JARs, complete relevant base/applied methods
and source/feature patches, original mappings, JDK and every input/library.
Use the real bootstrapped original registry and actual methods, never a
rewritten Java predicate as the oracle. CodeSource and whole-file guards must
refuse wrong origins before output, including under Python -O. Preserve all
original reference bytes and earlier exports unchanged.

Enumerate every original state with its actual numeric id, canonical block name,
complete sorted properties, default-state flag, air predicate, actual motion
predicate, leaf classification and fluid-state identity/emptiness. Enumerate
every actual Heightmap.Types value and directly invoke its original predicate
for each state. Inventory actual state/type counts rather than assuming them.
Compare Vanilla/Paper and existing FerrumC state identities exhaustively;
numeric alignment requires full property-level evidence. Unknown ids/properties
must never default to air, stone, solid or an arbitrary existing state.

Inspect complete Heightmap prime/update/getFirstAvailable/getHighestTaken,
storage representation and real callers, including heightmap lifecycle/status
requirements. Observe original methods on declared controlled real chunk inputs
for all six expected map kinds, if that is the actual census: air-only columns,
water/lava, waterlogged variants, leaves, plants/non-motion blocks, snow layers,
upper/lower slabs and build-height extrema. Capture first-available versus
highest-taken versus empty sentinels, negative minima, placement above/below,
replacement/removal, top-down rescans and raw packed representation. Clearly
label controlled chunks; these are not generated natural stage inputs.

If a real chunk caller cannot be initialized faithfully, report the exact gap;
do not use fabricated stage status or unsafe uninitialized objects to claim
lifecycle coverage. Predicate-only results remain separately scoped. Full
support/collision/light/occlusion/connection shapes need separate producers;
boundingBox/is_solid_cube is not a heightmap predicate substitute.

## Mechanical evidence and bounded implementation proposal

Deliver a lossless deterministic census/trace, explicit original output receipts,
complete source/callee review, an independent mechanical validator and fresh
normal/-O reproduction with byte equality. Validators reject malformed fields,
duplicate/missing states/types/properties, changed ids/predicates, wrong origin,
truncation and incomplete membership. Include positive controls. Expected
values must come from original calls, not FerrumC or copied formulas.

Propose a closed Registry-only static predicate producer with build-time input
limits, independent pins, identity joins, unknown refusal and exact table/native
capacity calculations. Then specify a separate World-only heightmap consumer:
six kinds if confirmed, typed state lookup, actual sentinel/packing semantics,
bounded scan/update work and lifecycle handoff. Do not change existing approved
limits. Separate future native targets from measurements. Document effects on
current flat callers and missing natural/Ore producers without modifying them.

Freeze only the new contract/tools/evidence after complete membership and
historical/outside-byte checks. Return exact hashes/counts, observed differences,
open decisions and smallest implementation split. Independent review precedes
runtime. Natural-world, Survival, whole-FEATURES, lighting, plugin, performance
and App acceptance remain separate tasks.
