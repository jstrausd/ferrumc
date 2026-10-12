# GEN-05.11 — independent cold Carver runtime review, round 1

12 October 2026. T3-owned independent reviewer for Root. **Verdict: FINDINGS.**

Candidate044 has one reproduced medium-severity accounting finding, **RR11-M1**. Successful cold numerical replay and native measurements passed the checks described below. They do not close the binding requirement that every counted operation must be admitted before it executes. No runtime acceptance, second World writer, integration, or App activation is granted.

## RR11-M1 — climate conversion executes before its charge

**Severity:** medium; blocks cumulative accounting acceptance.

**Frozen symbol:** `NoiseChunkSession::climate_sample`, `server/crates/ferrumc-world/src/density/chunk.rs:586`. The f64 density result is cast and assigned to the local f32 output at the expression starting on line 590. The finite check follows; `core_charge(&mut self.work)?` follows that check on line 610. The source is pinned in the complete source manifest and `review-input-pins.json`.

Revision2 §5 states, “Jede Charge liegt vor der gezählten Operation.” §5.1 expressly includes six f64→f32 charges in the climate-query bound of 772. Candidate044 instead evaluates:

```rust
*value = self.state.sample(/* original arguments */)? as f32;
if !value.is_finite() { /* typed error */ }
core_charge(&mut self.work)?;
```

When the density root consumes the last unit, the conversion and local assignment precede refusal of their own charge. This is a source ordering violation on defensive refusal. It is not evidence of successful numerical drift, a published partial chunk, meter truncation, or a public cap upgrade.

**Independent reproduction:** the private four-file observer overlay retains the original sample/cast/store, finite check and charge. A `#[cfg(test)]` observer immediately after the original cast reads the resulting bits and checks the actual private meter. It does not supply numerical answers, reset producers, or alter the charge algorithm. A genuine fresh router session at chunk `(0,0)` is queried through `sample_carver_climate(CarverClimateQuart::source((0,0), -8, -8))`. Actual constructor usage is 570916. The diagnostic sweeps post-constructor allowances 0–79:

| Allowance | Limit and used at completed cast | Cast result bits | Next error |
| --- | --- | --- | --- |
| 39 | `[570955, 0, 0]` | `3d8704f4` | Noise `ProducerWorkExceeded`, required `[1,0,0]`, limit/used `[570955,0,0]` |
| 59 | `[570975, 0, 0]` | `3dc4fc7f` | Noise `ProducerWorkExceeded`, required `[1,0,0]`, limit/used `[570975,0,0]` |

The observer suites reproduce both witnesses in Debug and Release. These are explicitly instrumented diagnostic binaries, separate from the unmodified candidate replay/native binaries. The diagnostic assertions intentionally confirm the violation, so their test exit is zero; that exit is not acceptance. Source ordering establishes the defect independently of compiler optimization of an unobserved local value.

Evidence is rooted at `.codex-tmp/gen05_11-runtime-independent-review-r1/`:

- Initial exact overlay: `overlay-manifest.json`, `overlay-final-sources/`, `independent-overlay-{debug,release}/{receipt.json,stdout.log,stderr.log}`.
- Debug stdout SHA256 `a58f588a1bc3d6949759afc41444695910d53e4d74587f76c4c2b6ed2375de6e`; stderr `099cf4ea2f27c6a55deda66fc49286cc1eec5dfc33f5e32f2aa60c1492f5c73c`.
- Release stdout `1564332f628c6e815794d1cb05a6b737164db5765787fd668bbcca47e3f4abeb`; stderr `caed503f34a76e9f8eb1f2cae951c6cee3e30f08145bc81e92c3501d82ee8518`.
- Copied initial Release diagnostic binary: `diagnostic-binaries/overlay-release`, SHA256 `e3803cbfdc1e53730177dc5a3cd4d4b4c90a462c08d46426d2cf00a43f8cdee2`; independently listed and the diagnostic repeated in `diagnostic-binaries/release-receipt.json`.
- Final six-test overlay and copied Debug/Release binaries: `followup/sources.json`, `followup/sources/`, `followup/results.json`, `followup/overlay-{debug,release}` and corresponding logs. Exact command/environment/source/binary/log identities are in that receipt. Both overlays are restored to the unmodified 1538-file baseline afterward.

The initial Debug overlay binary was subsequently overwritten by focused mutation builds; its initial argv/source/log evidence is preserved, and its binary identity is not reconstructed or asserted. The final exact-source Debug diagnostic reproduction has its own retained, pinned binary. Intermediate mutation binaries were not retained; their exact source overlays, argv, compiler/test logs, assertion failures and restorations are retained.

**Requested correction:** evaluate the f64 density root into a local, admit the conversion charge, then cast/store/check the f32 value. Preserve successful bits, original public caps, old expectations, exact nested errors and uninterrupted producer history. Add a regression that observes that a refused conversion charge prevents the conversion itself. Re-audit analogous explicit post-operation charge sites and the defensive nonfinite branch. Do not change structural limits or historical fixtures to hide the ordering failure. A subsequent review must use a new complete frozen candidate and the original brief plus this finding and the exact response.

## Authority and scope

The complete runtime assignment, original assignment/contract/review, correction assignment, revision2 response/contract and CLEAN preparation review, runtime delivery, AGENTS/CLAUDE, PAPER_REFERENCE, architecture/ADR0009, archived and candidate World invariants, and accepted GEN-05.3–05.10 producer contracts were read. All 38 new and 38 modified World files, affected producer success/refusal paths, importer/validator/allocation tools, and relevant real Java methods/callees and both applied patch layers were reviewed. Generated assets were checked by complete readback. The full unrelated Moonrise server patch is pinned as input authority; this is not a review or acceptance of whole-server integration.

`review-input-pins.json` pins the complete 362-file World authority, enumerated 76-file review delta, binding/historical documents, architecture inputs, original reference-source closure and instrumentation tools. Historical authority audits also check the original 1133-input inventory, original JDK/JAR/shadow guards and source-map closure. `authorities-initial.json` and `authorities-final.json` retain those results. Full byte/membership/readonly audits bracket private overlays; mutation clone audits compare every baseline/overlay member, not just modified files.

| Immutable authority | SHA256 / membership |
| --- | --- |
| Candidate complete package manifest | `574e54d8dafbd087d238d3d349f83369457664dec157e461ec2179129ce8c162`; 945 including manifest |
| Candidate source manifest | `243418db1406675450902b17a4f38c280b48a093faa7f91232e0cc2d1bb4334f`; 362 World files |
| Canonical World path→hash digest | `ab77a66d73a92882b661398931a99c6b7190984a1c49d99a13ac01586d735f19` |
| Candidate evidence manifest | `b01410b899a76bce7d63594f9eb1cef436afea1e8c723fb4a7087f4e15c7804a`; 579 |
| Runtime delivery | `b251eb841f1f1344d307379c4f64eb295c148dd2cbc18676df59aae3fb2d98e6` |
| Author summary044 | `65b4dfdfafd849d5c1492b86cef86e798b1f72f1239ef58ea3b4cf923c1fd9b7` |
| Revision2 contract | `ddd6c13188cfffc570ad80df9b9678d534e7c04859f23cfd6ee9e97f003ecbec` |
| Revision2 author freeze / inputs | `46a5d1dc02e913b0a72c0a979b1ddd393f46b52504b13871ab00f21e88f3f267` / `d4c9a8c60bf8ad0bac9d6f1d0c3fa8f0f8884c50c52a35caf4a045ed2921651c` |
| Revision2 CLEAN preparation review / freeze | `2c9a9c7af235ccbaa5bf0ea72e5465b8885d3d34a29707f264130382bb472d0a` / `2c7a2527b3173f933342b7db70fe0f0c36adcfe6fa72cc2b020c6d32e7da4f26` |
| Author tested47e accepted base freeze | `58ba13d6418133bb49a154682ba1d82533f63a5e6184c6c2a8b07c6c15a47a75`; 1490 files |
| Fresh current356 accepted base freeze | `3788f6be81effe6d4d06db3bf4380319844ed3738dbf80dc90e62a119041d6ae`; 1499 files |
| Current356 + frozen World + delivery | `628bce18102403a0cef341ea0b5000eb29deaf21cea9b1632fd4fdebc3a085a3`; 1538 files |

Author whole validation belongs to main `47eadc4c14813c201184b552a87ca7b9c6f97db4`. My checks use the fresh archive of main `356670f7714bc42f2626dd68809e6fd1975a5d96`, under `build/`, with a separate target and locked/offline Cargo jobs2. The server/runtime/dependency bytes of these accepted bases are independently equal; eleven changed paths are docs/CI timeout. No shared-WIP build or Git operation was used. All 202 historical fixture/reference files retain their original bytes; no historical repair occurred.

`shared-before-final.json` records the 1175 base-owned current outside-World paths and their shared byte pins. The pre-existing difference from the current accepted archive is `docs/IMPLEMENTATION_BOARD.md`; it was neither changed nor used as the private build baseline by this reviewer. `shared-final.json` records the final comparison. After the first freeze, an external update changed only `docs/IMPLEMENTATION_BOARD.md` from `6d027995f6ca4843f24ec3b2b625140238cb89722e88f210d7116a5af71fe2d0` to `daeb7f11003b52735da9f37b1c1da8064a1b0638fff86bef891e19b8fc381fb9`. `external-drift-after-freeze.json` records it. A later shared update changed `scripts/check-forbidden-claims.py` from `e61094fee2b2530b2840749e1894e0702e507d8fda3430ab077d934189fc0f4c` to `03169058de133b7b34f7a4742e8193e7adb5cad7a40ae16f6d7017008855944f`; `external-drift-after-second-freeze.json` records it. Candidate/accepted-base bytes and original authorities remain unchanged. These live shared-workspace updates were not written by this reviewer and are outside the immutable private tested combination. The final verification records any further outside differences explicitly; it makes no claim that concurrent Root work is globally frozen. Root's author-base outside count is 1166; it is a different base and is not silently substituted for the current count.

## Cold numerical composition and instrumentation

Fresh focused unmodified Debug and Release `--lib carver` checks passed (`carver-debug/receipt.json`, `carver-release/receipt.json`). They execute real Surface production first, then the Carver production composition. The complete 71 admitted Paper groups compare complete FINAL states/digests, mask, heights, fluid set and start/RNG/source/carver trace. Aggregate evidence is `GEN0511_REPLAY [71, 42117, 355, 157933] Vanilla-different=25`. The two refused groups remain typed Empty. The 25 distinguishing Vanilla groups retain their control labels.

Source inspection supports genuine direct climate/selector/zoom: every admitted group performs all 289 source queries in X-outer/Z-inner order; consumed fix-ups are interleaved in actual carving chronology. The private derived halo/Y−17 path bypasses public Stored admission without replacing the evaluator or lazy memo. Temperature uses that memo. Nonconsuming rules do not issue biome queries. Only the rule column/block context is reset. Aquifer location/status caches and the stale Lava scheduling flag persist through the invocation. There is no fixture-fed production start, biome, state or Aquifer answer and no separate test-only numerical success implementation. Test observation hooks report production decisions.

The original f32 expression/order requirements, complete SIN65536 rawbit import, all 63291 start tests, boundary clipping, extra Cave branching, Canyon112 strict thickness/no child recursion, replacement tag, mask-before-write, barrier/tip rescan, mycelium/no-rule and fluid edges were inspected and exercised by focused replay/geometry/control tests. `structural-assets.json` independently checks all 65536 imported bits against original observations and all 52 tag members; original SIN little-endian digest is `cdfaec6870788e193dff3f1ddfa3ca7d3f1a897042d78365a1fe3a433be3627d`.

Validator confidence was challenged independently:

- `validator-controls/results.json`: two positive routes and seven digest-repinned semantic negative controls, rejected in normal Python and `-O`.
- `additive-{normal,optimized}.json`: 15 files; eleven original fixture files unchanged, three additions with authoritative provenance; 71 full/two typed refused and 25 Vanilla controls.
- `observer-{normal,optimized}.json`: all 30 backend observation TSVs equal the frozen original outputs; eight semantic mutants rejected in both modes (omitted/extra/out-of-order source, source biome, fix-up quart/biome, omitted lazy and eager extra request). Across all 73 observation groups: 21097 source calls, 355 fix-ups and 355 consumed lazy queries; admitted source calls are 20519.

These are fresh independent checker/control executions over frozen genuine Java observations. I did not rerun a fresh Java export and do not label these checks as one. The reviewed Java/source/patch instrumentation plus positive/negative controls did not leave an instrumentation objection requiring such a regeneration in this round.

## Accounting, errors and state

The structural bounds were reviewed against implemented graph/key/spline construction, closed selector tree/rule traversal and finite loop bounds. `structural-assets.json` separately recomputes arithmetic and independently derives the 297 preliminary columns/315 location-status misses from all thirteen original Aquifer offset pairs. It is an arithmetic/asset check, not a substitute for the source recurrence audit. Corpus maxima are never used as producer admission authority.

| Authority | Bound |
| --- | --- |
| Geometry R | 63912717682 |
| Bind | 90 |
| Noise constructor | 117744161352 |
| Prepare reservation inside Noise total | 117744178367 |
| Noise total | 321566836740286847972889004023138800132439; limbs `[117840306519,0,945]` |
| Noise remaining | 321566836740286847972889004023021055954072; limbs `[96128152,0,945]` |
| Selector | 18716174375 |
| Rules | 45120000 |

The closed Noise construction/query proof includes 945P. Public caps remain unchanged. The private unsigned192 limit/used and segment bookkeeping retain uninterrupted cumulative history; segment rollover supplies no new allowance. RR11-M1 prevents declaring all pre-operation charges correct despite those consistent bounds.

The private wide-error paths were inspected through constructor/basis/RNG/Cold/Keys/Spline, density/Marker/ChunkState/preliminary/climate, Aquifer, selector and lazy rules, including error payload translation and public regressions. The unmodified focused suites execute actual fault adapters. Independent observer suites use 497 Python arbitrary-integer add/sub/order cases, full refusal preservation, 64/128 carry, 192 overflow, seven admitted charges followed by refused eighth charge, and normal/short-segment controls. An actual constructor sweep covers 42 allowances, actual N−1/N/N+1, a starting offset crossing 128 bits, segment size1 and exact full error limbs. Final follow-up probes use independent literal bounds for bind89/90/91, totalN−1/N/N+1 and production remainingN−1/N/N+1; the remaining refusal starts Published, ends Empty, retains Ready/history, and unchanged input stays Published. There is no narrow fallback, hidden reset or truncated error in these exercised paths.

Preparation and carve admission order, all individual/combined mismatches and defensive failure paths were inspected. Real producer tests cover Ready→Running→Complete/Poisoned, unchanged Ready state for pre-run refusal, poisoning after execution failure, and no reuse after Complete/Poisoned. My additional production probe publishes output, refuses insufficient geometry budget to Empty without advancing history, retries the same Ready invocation successfully, then refuses its second use to Empty. Original Surface publication/content remains unchanged.

The private finite-f64 cache repair is a checked finite-bit encoding with a distinct empty sentinel; nonfinite/type admission remains typed. My cache probes check 20480 stratified encodings including exponent extremes, signed zero and nonfinite patterns, plus one million deterministic random bit patterns. Finite bits are preserved exactly; invalid sampled values are refused; cache8B and public rule session400B remain literal. Original expectations were preserved.

## Native behavior and RR10 follow-through

Both profiles freshly execute all seven author probe categories using binaries built from the exact current-main combination: Carver, full simultaneous native census, RR10 Binding/Material/Session/UnclassifiedState, temperature, Stored, rules and Surface. `final-probes-{debug,release}-044/results.json` pins argv, binaries and outcomes. Each external interceptor has the positive control of exactly one 4096B allocation and one free. All prepared drive/query/evaluate phases show zero allocations **and zero deallocations**. Constructors are measured separately. There is no warmup before first invocation.

The Carver native probe drives 21 successes, 21 actual already-Published output refusals and 13 constructor refusals. The census includes simultaneous input producers, binding, prepared run and output, borrowed/owned values and capacities, selector tree/Arc header, noise graph/cache/scratch/preliminary capacity, Canyon stack and static SIN. Requested-heap retained is **4175577B / 2638 objects**, peak **5388857B / 5084 objects**, independently reproduced in Debug and Release. Static SIN262144B and value/stack layouts are separate categories; borrowed capacities are not summed twice. Partial825488B is not whole memory. These are requested allocator bytes for this exact cold binary, not RSS, allocator metadata, latency, TPS or all-seed performance. Stack-write observations retain their tool-defined scope.

`ledger-controls/result.json` independently exercises a complete32768 collision chain, tombstone deletion/reinsertion, missing-free refusal, byte/object peaks and explicit exhaustion exit90. The bounded observer does not silently undercount collisions or exhausted capacity.

Production layout integration passes separately (`production-layout/`): binding1560B, Prepared1360B, Noise704B, Selector80B, rules400B, Error104B, Report104B, output value1104B, column value576B and Surface value1088B. Internal compile-time literals include meter56B, SampleWork64B, direct provider48B and Aquifer144B. Retained columns **10816B**, Surface **406592B** and output418896B are distinct from production value layouts.

RR10-H1–H5 are supported at implementation/runtime level in this candidate: actual y319 scan, distinct lava/water erosion/frozen edges, literal retained sizes and new layouts, external allocation coverage for all four named refusal categories, and genuinely Published→Empty output refusal. My new exact-clone mutation controls have unmutated positive passes and assertion-level kills:

| Mutation | Production change | Result |
| --- | --- | --- |
| M13 | Start `scan_top` at318 instead of319 | exit101; actual top −64 versus expected320 |
| M14 | Frozen water predicate accepts all fluids | exit101; intended frozen assertion detects Lava/water mismatch |
| M15 | Eroded water predicate accepts all fluids | exit101; intended eroded assertion detects Lava/water mismatch |

`rr10-controls/results.json` retains exact argv/log hashes; complete baseline/overlay manifests and each changed driver are retained. No compile failure is counted as a kill. Every mutant is restored and verified against all1538 files before the next overlay.

## Validation identities, preserved attempts and freeze

`author-evidence-audit/result.json` independently validates the frozen author final command argv/log hashes/counts and all fourteen final probe binary/measurement identities. Author47e whole suites report415 World passed/7 explicitly driven ignored probes in each profile,1550 consumers passed/1 ignored, and successful workspace fmt/all-target Clippy/build and World rustdoc. Those are author checks; I did not rerun whole consumer suites simply for green output. Own unmodified current356 evidence consists of meaningful focused replay/error/geometry checks, production layout and complete cold probes. Own four-file diagnostics and three separate one-file mutations have explicit overlay scope.

The copied unmodified own World probe binaries are SHA256 `4f2867e5dcf89172e3a790fd8d2fa762b51f1901902c5b0cd45083b56028c118` (Debug) and `143f320e59ab73d9907753245e1bedc21c67d5bd15d235fa61b34060e8f35148` (Release). Baseline replay stdout hashes are `64f8b61dfaf0797194644b6a6f67c4c2b9c8b29937cda6ae645428e2e267982c` / `bcc3c1976d42dcc3c76ad0cc2f989b7147a4133228c3b0a1cc7637f42a96b1d5`. Additional profile-specific binaries and all final observer identities are in the complete reviewer evidence manifest and receipts.

Preserved reviewer failed/superseded attempts are separate from candidate defects:

1. Readonly private base initially prevented an overlay write: `failed-build-intake/` and `failed-intake.txt`; only private-copy permissions were corrected.
2. An overlay was applied while a baseline audit was running; the audit rejected membership: `failed-overlay-order.json`. Restored baseline, corrected audit and exact reapplication receipts are retained.
3. Wrong integration target `surface_column_layout` exited101: `allocation-binaries-debug/`; corrected `carver_runtime_layout` succeeds in `allocation-binaries-debug-corrected/`.
4. First author-evidence audit assumed every legacy phase was named DRIVE and stopped at KeyError. `failed-author-evidence-audit/` preserves that script/error; corrected audit validates EVALUATE/QUERY as appropriate and passes.
5. The first post-freeze verification rejected the external task-board drift. `delivery-superseded-before-external-drift/`, `freeze-receipt-superseded.json` and `failed-post-freeze-external-drift/` preserve the complete initial freeze and failed verification. The second freeze/post-verification attempt likewise rejected a later external claims-script update, preserved in `delivery-superseded-before-second-external-drift/`, `freeze-receipt-second-superseded.json` and `failed-post-freeze-second-external-drift/`. The report/final freeze were revised to state these external differences explicitly and distinguish live-workspace snapshots from immutable tested inputs; the candidate and finding did not change.
6. A few wrong-cwd/missing-path source reads produced no source mutation or candidate check. They are not represented as candidate failures.

The complete new review sources/evidence/input pins and this report are frozen read-only under `.codex-tmp/gen05_11-runtime-independent-review-r1/delivery/`. `freeze-manifest.json` pins the report and all retained evidence; `freeze-receipt.json` records exact final manifest/report hashes and counts without a self-hash cycle. Bulky build/target/restored-clone and bytecode exclusions are explicit, with source equality/restoration or an excluded-artifact inventory as appropriate. Failed attempts and final immutable inputs remain preserved in the permitted reviewer area. `final-input-audit.json`, `authorities-final.json` and `post-freeze-verification.json` record unchanged author package, references and bases, plus the shared outside comparison and explicitly recorded external shared-workspace drift. No production, dependency, lockfile, historical documentation/fixture or unrelated correction-scope file was edited.

## Historical objections and residual gates

CR11-B1 and CR11-B2 were closed **at preparation level** by Root's accepted revision2 response. B1 required genuine direct climate/selector/zoom, all289 ordered source calls, interleaved consumed fix-ups, private halo/Y−17, shared lazy memo and no session reset. The runtime evidence above supports that corrected implementation. B2 required structural constructor/Noise/selector/rule bounds, checked private unsigned192 cumulative accounting and exact errors without changing public caps. Its structural/wide-history portions are supported; RR11-M1 is an implementation violation of its precharge requirement. Preparation closure has not been promoted into runtime acceptance. Original policies and accepted responses remain binding.

The returned verdict is **FINDINGS**, pending correction and an independently reviewed newly frozen candidate. This cold producer does not establish warm/thread-history parity, practical all-seed timing, actual generator-to-Chunk/biome/heightmap/light/persistence integration, features/structures/Nether/End, Survival, plugins or App activation. Natural-world activation remains a separate required operator milestone owned outside this review.
