# GEN-06.2 — closed ore placement primitives, queued runtime assignment

Root design disposition, 12 October 2026. The independently reviewed GEN-06.1
preparation is accepted at **CLEAN_CONTRACT / CLEAN_ORACLE** scope. CR61-B1/B2/B3
are closed for the writer-free primitive. H1's original source description is
corrected only in the new review; historical bytes remain unchanged. H2 and the
region/Section/core/FEATURES gates remain open.

**Queued, no execution owner assigned yet.** GEN-05.11 retains the exclusive
World writer. Root will dispatch this ticket only after that writer is finished,
its candidate is independently reviewed and the integration basis is decided.
At dispatch Root supplies a newly verified accepted-main archive and manifest.
Neither the mutable shared World tree nor this preparation review authorizes
starting another World writer. No App, Sim, Storage or Registry activation.

## Binding inputs and Root decisions

Read AGENTS.md, CLAUDE.md, World INVARIANTS, architecture and PAPER_REFERENCE.
Read the complete original GEN-06.1 task/contract/review, Root revision-2 task,
[revision-2 contract](GEN-06_1_ORE_PLACEMENT_REVISION2.md),
[independent revision-2 review](GEN-06_1_ORE_PLACEMENT_REVISION2_REVIEW.md),
and all referenced accepted producer contracts. Inspect actual full original
methods/callees and both relevant patch layers; preserve their Paper/Vanilla
differences. Paper Build60/commit29c8822d, Minecraft1.21.8/protocol772.

| Accepted preparation pin | SHA256 |
| --- | --- |
| Revision-2 contract | `24ab2da586620009f25bfbfd99e433215d7e8698ec76a62eaff03ec03ec74eb8` |
| Revision-2 author freeze, 366 files | `e44f7199a3f222007e9759a6fff69cf0ab030008e6adcbe003b8d03da81315a7` |
| Independent revision-2 review | `1b2220e923033a3bf22843a7de3a86f7f056850c2e9715f123501e8b5f100027` |
| Independent freeze, 494 files /307,996,259 B | `dd95a0be1baa4e176857a518978543d86427a2461e8cf796e00a365b1f6e63a4` |

Root reverified every independent frozen byte and accepts the proposed targets:
retained catalog **16,384 B**, total construction scratch **16,384 B**, simultaneous
new peak **32,768 B**, old catalog plus new peak **49,152 B**, cursor **512 B**.
These are design limits requiring actual native capacity/stack/temporary/error
measurement, not measured facts or an RSS promise. Four scratch partitions must
include headers, capacities, alignment, parser/IR/SHA/compiler locals and callstack;
an additional full 16-KiB arena plus external locals violates the limit. If actual
construction cannot satisfy these targets, report the concrete layout and smallest
correction to Root before candidate freeze. Do not silently raise caps or omit work.

## Exclusive scope at later dispatch

One World-only production ticket:
`server/crates/ferrumc-world/**`, new
`docs/agent-tasks/GEN-06_2_CLOSED_ORE_PLACEMENT_PRIMITIVES_RUNTIME_DELIVERY.md`,
and `.codex-tmp/gen06_2-runtime/**`. No other production crate, Git, agents,
dependency/lockfile changes or historical contract/fixture/freeze mutations.
Only Root integrates. Private builds use the dispatch archive plus whole World
overlay and separate target, Cargo jobs2/locked; shared Storage/Sim WIP excluded.

Implement the whole revision-2 §§7/9/11/12 primitive contract:

1. Closed checked World-owned IDs and complete 57-resource Paper binding:
   30 Placed /24 Configured /3 Tags, all120 modifiers, concrete properties/state
   ranges and the complete **164 global indices**, including non-ore entries.
   Paper resource bytes26,987/max721; Vanilla26,930/max720 differs by one LF per
   entry and is only a test discriminator. Verify exact original sources before
   parsing; no fixture authority, ore-prefix discovery or alphabetic/ore-only sort.
2. Obligatory BindAllowance1..=2^20, complete cumulative before-work accounting:
   **Paper N869,557 /Vanilla N869,258**. Implement each actual loop/debit in its
   specified order: fallible allocation admission, initialization, source headers,
   SHA input/padding/schedule/rounds, byte/digest comparisons, POD parser arena,
   one decoded-string pool, schema/duplicate checks, references, grouped Registry/
   property reservations, typed IR, every index/pair/target/census check, pre-reserved
   cleanup and final publication. Group reservations must precede real inner calls.
   No free library hash, second parse/decode, uncharged content loop, hidden growth,
   or census after publication. N-1 refuses final publication and keeps the old
   catalog; N/N+1 publish once with used=N. Old release follows atomic publication.
3. Private genuine BitRandomSource/WorldgenRandom decoration adapter using exactly
   one RNG core and original Java-long/f32 arithmetic. Preserve seeding and each
   bounded-int rejection. The Paper double profile uses l2f/fmul/f2d and can yield
   1.0; Vanilla l2d/dmul remains a discriminating test. Existing public GEN-04 APIs/
   fixtures/caps stay unchanged; direct-Xoroshiro APIs are not a renamed wrapper.
4. Lazy sealed session, one immutable cumulative RunAllowance1..=2^20, states
   Fresh/Running/Pending/Complete/Poisoned. Count/Rarity, X then Z then Height;
   Pending is unresolved continuation, never a biome pass or Ore success. Exclusive
   borrowed lease and private dispatch/completion identity/sequence prevent a second
   next, double completion, foreign catalog/key/identity, reseeding/second RNG,
   reset/upgrade or callback/bool authority. Charge before every core mutation;
   refusal preserves RNG/successful_core, prior admitted prefixes stay diagnostic.
   Java getCount-before-delegation remains separate from successful-core count.
5. No production biome or Orewrite consumer in this ticket. A private empty
   production consumer enum with a separate consume callee is acceptable; cfg(test)
   variants must use the real private dispatch/lease/meter/completion kernel.
   Option::take disarms only after all fallible/panic-capable completion actions.
   Drop, consumer failure, completion refusal and unwind poison; mem::forget leaves
   Pending until the next call terminally refuses/poisons without RNG/work mutation.
   No public success stub or separate testcursor that bypasses production handoff.

## Required complete runtime acceptance

Preserve and mechanically import all original rows/extra rows and source pins;
compare every token/bit in Debug and Release. All48 seeding/wrapper rows,
60 Height/52 Count endpoints, rejection/Doublemax and128 review-extra rows remain
required, not just final positions. Begin8, Count0/Rarityfalse11 and interleaved
seven-candidate ore_dirt80/Core32 are FerrumC accounting expectations; expected
drawbits come from genuine original lazy execution.

Run the full revision-2 §11 matrix in the **actual** private kernel: parser/schema/
UTF8/truncated/trailing/unknown/duplicate/escaped-duplicate/resource/index/property/
rule/provider/anchor failures; production digest precedence; original lenience
versus stricter closed admission; sizes/chances/counts/heights/overflow; source,
arena, depth, string and entry limits; every Bindprefix/group reservation; allowance
0/1/max/max+1; N-1/N/N+1 at seeding/rejections/Count/Rarity/Emission/Dispatch/
Completion/End; raw RNG state and successful-core preservation; foreign identity/
sequence and terminal cases. Include compile-fail exclusive borrowing and move
controls; source similarity to the review sketch is not implementation evidence.

Capture native literal layouts and simultaneous capacities/stack/temporaries,
each partition and aggregate N-1/N/N+1 using private controls. Execute cold-process
Debug/Release allocator probes for first successful bind and every refusal class,
including real fallible-reserve refusal, with independent positive4096-B control.
Construction intentionally allocates within admitted limits; report actual calls
and bytes. Claim allocation-free candidate/session operations only after measuring
them. Source/static/borrowed/old/new/allocator overhead stay separately identified.

Announce whole World candidate before complete checks, request Root's exact archive
if integration main changed, then whole World Debug/Release, strict all-target
Clippy -D warnings, fmt and rustdoc -D warnings. Required consumer checks use that
exact accepted combination. Freeze source/evidence only after all checks and fresh
membership/outside-byte verification. Independent runtime review follows separately.

This producer implements no Ore blockwrite/Region/Biomeconsumer, CARVERS acceptance,
FEATURES/Structure driver, multichunk adoption, Naturalworld/Survival/Java-plugin/
lighting/Nether/End activation or performance guarantee. GEN-06.3 original Section/
Bulk/core chronology and H2 complete region caps remain separate prerequisites.
