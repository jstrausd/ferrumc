# GEN-05.9 — overworld surface rules work order

Root work order, 11.10.2026. Continues the connected World chain after the
accepted GEN-05.8 stored biome resolver. This ticket covers the original
overworld surface-rule data and their evaluation for one block position. The
complete `SurfaceSystem` column driver (stone depth, water height, surface
level, bandlands/iceberg extras and writing into chunk sections) follows as a
separate ticket. World remains the only production owner.

The next World slot first produces a concrete contract and a genuine original
oracle; runtime follows their independent Root review. Preparation owns a new
`GEN-05_9_SURFACE_RULES.md` and `.codex-tmp/gen05_9/**` only. Production stays
read-only during that phase. A later runtime authorization owns only
`server/crates/ferrumc-world/**` and new runtime evidence/delivery. Root owns
Git, the board, integration and cross-crate consumers. No new dependencies,
other agents or edits to historical GEN-05.x artefacts.

Read the complete real Vanilla 1.21.8 and Paper build 60 implementations and
both patch layers for `SurfaceRules` (every `ConditionSource`, `RuleSource`,
`Condition`, `SurfaceRule` and their codecs), `SurfaceRuleData` (overworld),
`SurfaceSystem.Context` and its lazy per-block/per-column state, the
`VerticalAnchor` resolution, the noise and positional-random factories the rules
use, and the `NoiseGeneratorSettings` overworld `surface_rule`. Pin whole
sources, the actual original classes/JARs/libraries/JDK, the built-in data and
the accepted FerrumC producers this ticket consumes (GEN-04 noise/random,
GEN-05.4 material, GEN-05.5–05.8 biomes).

The concrete contract must settle:

- An immutable, bounded compiled form of the complete original overworld rule
  tree with exact node/condition order, sharing and depth limits, built from
  the actual original data without hand transcription. State the data source and
  how drift against the original is detected.
- Every condition's exact inputs and evaluation order: biome (through the
  accepted GEN-05.8 resolver only), noise thresholds, vertical gradients and
  their positional random, Y/water/stone-depth/steep/hole/temperature/abovePreliminarySurface
  predicates, and their original caching/laziness semantics where they change
  results. Keep block, quart and resource IDs distinct.
- The explicit per-block and per-column context the evaluator consumes, which
  later the column driver must produce. Name every field, its domain and who
  produces it; this ticket does not compute the column itself.
- Result type: the selected block state or "no rule", with typed FerrumC
  refusals for missing/inconsistent inputs instead of default blocks.
- Exact retained/native capacities, evaluation work bounds and meaningful
  N-1/N/N+1 admission tests; allocation-free evaluation with a positive
  control. No live TPS, RSS or full-server speedup claims.

The oracle must invoke actual original rule/condition objects with genuine
original contexts, not an independent rewrite of their logic. Cover every
condition and rule kind present in the overworld tree, both sides of every
threshold, vertical-gradient boundaries, negative coordinates, height ends,
water/no-water, steep and hole cases, temperature-dependent branches, biomes
selected through genuine stored biome lookups, and Vanilla/Paper numerical
modes. If driving an original condition requires column state the original only
produces inside `SurfaceSystem`, record exactly which original code produced
it; do not fabricate it. Retain both originals' outcomes and exceptions.

Provide reproducible fresh-output runners, exact input/output manifests and
guard checks with normal and Python `-O` behaviour. Rust expectations must be
mechanical projections of genuine original outcomes, without numerical
expectation formulas. Keep policy-refusal and malformed-input tests separate
from original behaviour. Root independently reproduces and reviews the contract
and oracle before authorizing production.

Carried condition from the GEN-05.8 runtime review: the next World runtime adds
a test that pins the two-phase halo refusal order of `stored_biome`
(`GEN-05_8_STORED_BIOME_RESOLVER_RUNTIME_REVIEW.md`, R-A1).

Out of scope: the column driver and writing surface blocks into chunks,
carvers, features, structures, lighting, natural-world persistence or
activation, Nether/End rules, Survival and Java plugins.
