# GEN-06.1 — independent correction review, round 2

Root assignment, 12 October 2026. World generation remains the operator's
priority. Review the corrected preparation before GEN-06.2 runtime work.
Prior verdict: BLOCKING_CONTRACT, CR61-B1/B2/B3. Original-method evidence and
builtin inventory were reproduced; the objections concern construction,
single-stream ownership and inaccurate semantic/coverage statements.

## Complete original brief and exclusive scope

Read AGENTS.md, CLAUDE.md, PAPER_REFERENCE, World invariants/architecture,
accepted producer contracts, the complete [original task](GEN-06_1_ORE_PLACEMENT_TASK.md),
[original contract](GEN-06_1_ORE_PLACEMENT.md),
[first review](GEN-06_1_ORE_PLACEMENT_REVIEW.md),
[Root correction task](GEN-06_1_ORE_PLACEMENT_REVISION2_TASK.md) and
[revision-2 contract](GEN-06_1_ORE_PLACEMENT_REVISION2.md). Inspect actual complete
base/applied original methods/callees, both relevant patch layers and the real
accepted Registry/RNG/parser implementations needed by the proposed primitive.

Write only the new
`docs/agent-tasks/GEN-06_1_ORE_PLACEMENT_REVISION2_REVIEW.md` and
`.codex-tmp/gen06_1-r2-independent-review/**`. Author sources/evidence, historical
fixtures/freezes and all other scopes are readonly. No production, dependencies,
Git or further agents. Private test-only probes may use an explicitly verified
accepted archive copied to your own area; enumerate any overlays/outside bytes.
Never build mutable shared Storage or private unaccepted Sim candidates.
Carver correction has its own review; GEN-05.11 is still unaccepted.

## Pins and prior objections/responses

Revision2 contract SHA256
`24ab2da586620009f25bfbfd99e433215d7e8698ec76a62eaff03ec03ec74eb8`;
366-file /73,591,041-B freeze
`e44f7199a3f222007e9759a6fff69cf0ab030008e6adcbe003b8d03da81315a7`.
Root reverified every frozen file and contract byte, not claim correctness.
Original contract `a12b58d1bffbfd0ad918afe624fec7f32cefcc117d4a63a31c7ba431ab46b082`;
original 1,147-file freeze
`720c775e6defd843996de0229687a9fe9882fe3740ba0921368812f0ab73a269`;
first review `48737c693cf6bdcb6db68e62e069a7df9cd47d82591b5b109fb0357c90bc59a0`;
164-file review freeze
`336a8e7910acabfde0c7afcf29e70d84cb672b937f306b7df5652d231dfbb0fe`.
Independently verify all new and historical pins/membership, including 1,728
original inputs, 190 whole source/patch copies, 894 resource bytes, JDK and
324 accepted World files. Separate author execution from your own observations.

CR61-B1 previously lacked mandatory bind work/simultaneous scratch and concrete
ownership between candidate/consumer. Revision2 proposes complete 57-resource
binding, Paper bytes26,987/largest721 versus Vanilla26,930/720 (one extra LF per
Paper entry), 1,684 nodes, 120 modifiers, 12 tag references and all164 global
indices. Paper successful bind debit869,557 /Vanilla869,258, separate
BindAllowance1..=2^20. It proposes a 16-KiB staging catalog, 16-KiB scratch in four
partitions, 32-KiB new peak /49,152 B with old catalog and 512-B cursor target.
Actual native capacities/layout remain a runtime gate. Root's preferred design
is these proposed construction targets if your independent review finds them
bounded and feasible; do not treat this preference as measurement or runtime
authorization, and do not force a smaller target without evidence.

The same revision specifies one immutable RunAllowance1..=2^20, one RNG owner,
Fresh/Running/Pending/Complete/Poisoned, an exclusive borrowed Pending lease,
sealed internal dispatch, terminal drop/failure/panic/forgotten-lease behavior,
and no external callback/bool acceptance. GEN-06.2 has no production biome or
Orewrite consumer. cfg(test) must exercise the real handoff. Later2^32 region
allowance cannot upgrade/reset this session. Begin/seeding debit8, Count0/rarity
refusal11, seven-candidate ore_dirt plus interleaved consumer floats80 are new
FerrumC accounting expectations, not original Java work counts.

CR61-B2 response: FeatureSorter discovery is equals/hashCode; only later per-step
lookup is identity. hasFeature is equality on canonical records. Preserve genuine
equal-clone/self-cycle controls and every164 original index, not an ore-only sort.

CR61-B3 response: 60 controlled placements contain no S-events. Public returned
draw/proxy guard/height/biome/chunk events and net snapshot deltas are observed;
direct Section/Bulk reads/writes/acquire/release and complete core rejections are
not. The16 neighbor callback probes are separate. Full Section/core chronology
stays a GEN-06.3 gate, not falsely a completed writer-free primitive prerequisite.

CR61-H1 source dispute: the frozen original helper actually contains
`Math.abs(bx(a[0])>>4)<=1&&Math.abs(bz(a[0])>>4)<=1`; Root independently read it.
The first review's `abs(block)>>4` description is inaccurate. Check whole-file
pin and18 new signed-chunk boundary controls, including a separately labelled
wrong-order negative control. Preserve the old review bytes; record the correction
in your new report, never claim the historical code was fixed by a mutation.
CR61-H2 remains: input+copy state payload alone7,077,888 B; old4-MiB region claim
is withdrawn, later full region/adoption cap unresolved and outside this primitive.

## Required independent review and experiments

- Recompute exact Paper/Vanilla resource census directly from original ZIPentries;
  verify LF-only differences, semantic identity, SHA block padding, node/edge/key/
  string/property/tag counts and immutable global indices. Check complete closure,
  actual Registry lookup bounds/properties/ranges and absence of fixtureauthority.
- Audit every proposed bind debit and actual intended control flow: byte hashing/
  comparison/decoding, padded schema/string slots, all reference/IR/state/tag/index
  loops, paired duplicate/index/target checks, allocations and pre-reserved cleanup.
  Independently recompute sums. N-1 must fail at final publication and preserve
  the old catalog: no census or fallible work may remain after publication.
  Group reservations must bound real calls; unused reserve is an explicit policy,
  not an assertion that a runtime inner call was measured. No uncharged content-
  dependent loops, hidden second parse or library growth may remain.
- Inspect whether four scratch partitions, POD parser arena, decoded string pool,
  typed IR and fixed staging catalog are concretely implementable with before-
  growth fallible admission. Source/static/borrowed/old/new/layout/temporary bytes
  stay distinct; no stack or allocator capacity hidden from simultaneous peaks.
  No new dependency is implicitly approved by this review.
- Review lease/borrow/dispatch semantics, Option::take and panic/drop/forget paths
  critically. One cumulative core meter includes each rejected bounded-int draw;
  refused charge mutates neither RNG nor successful-core count. Original Java
  getCount before delegation stays a separate diagnostic. Discriminate second
  next, double completion, consumer reseeding/second-RNG/reset/upgrade and forged
  identity/sequence. The future consumer gap must remain explicit without making
  the actual private handoff untestable or inventing a production success variant.
- Freshly reproduce changed/new evidence normal/-O as appropriate: original
  seed/double/rejection/provider rows, lazy interleaved128 review rows and boundary
  controls. Use genuine original expectations. New independent census/model/
  framing/semantic mutations must fail; old fixturebytes remain unchanged.
  Repeat unrelated whole-runtime checks only for a concrete new concern.
- Confirm original feature order including non-ore entries, Stored feature biomes
  versus Direct Carver source, original Paper Double=1, exact profile restriction,
  deferred Section/core/region/FEATURES gates and every later obligation are kept.

Finish with CLEAN_CONTRACT/CLEAN_ORACLE only if B1/B2/B3 are truly closed and a
bounded implementation brief is concrete; otherwise specific BLOCKING_CONTRACT/
FINDINGS with reproducer and smallest correction. Recommend Root disposition of
scratch/peak targets separately. Freeze the new report/evidence after fresh final
integrity checks. No native runtime/allocation/RSS, Carver, generated FEATURES,
natural-world/Survival/performance or App activation acceptance.
