# DUR-03.3b — independent startup receipt reuse runtime review

12 October 2026. **Verdict: CLEAN_RUNTIME_REVIEW.** The medium-severity R6-B2 startup/liveness gate is closed on the frozen whole 93-file Storage candidate. The original public probe-8 database actually closes and reopens at its original 1-GiB relation allowance. No new blocking authority, durability, runtime or preserved-contract objection was found. This is an independent review result for Root to integrate; it authorizes no Storage, Sim or App activation.

## Review basis and integrity

The binding review assignment is `DUR-03_3b_STARTUP_RECEIPT_REUSE_REVIEW_TASK.md`, including the complete Root prompt. The supplied docs-only HEAD is `18d510e1`; no Git command was used to inspect or change it. The build basis is deliberately accepted main `4225c21cfcb648a075f807b32e586276fe6661c6` plus the whole frozen candidate and new delivery, never the mutable shared checkout.

Required reading completed: AGENTS.md, CLAUDE.md, candidate Storage INVARIANTS and README, architecture/storage, ADR-0004/0007/0009, PAPER_REFERENCE, complete original DUR-03.3 contract, original source/design evidence and contract review, all three runtime reviews and their complete original-delivery responses, DUR-03.3b task, review assignment and new delivery. All six changed files and two new tests were read completely, along with the affected complete startup/proof/reader/budget/open/write paths and real redb 2.6.3 callers/callees. Paper pins were verified as historical input integrity, not differential acceptance.

| Input | Verified SHA256 |
| --- | --- |
| Original contract | `2edfc588fd8d7e96753f6beb4bb7c2ad50a064e4c266681ab5ff94550c4342ab` |
| Original 16-artifact design freeze | `3710c57265084f9a6cd59d48fc77b9340fcd255bd0f235545c7ca639b92de92f` |
| Original 1,224-file archive manifest | `c148936b29cc58d7ac25b2904618523c3b3a163476a2bf380b1d00fadf07c7b0` |
| Round-6 91-file candidate manifest | `d21db90ec56958c965c2a8fb288d21b82ad5af22c8bda9cd0d471cd894abd0d7` |
| Round-6 38-artifact evidence freeze | `0335589a6f659d79a0750e53487d960356ba567138e6829f33ad8dc8f9b73e0c` |
| Review-round-3 29-artifact evidence freeze | `88ab8eafd921d06a5f2de215a4fe010dd800a18e3e22cf89490c4638633406fd` |
| Corrected author baseline, 1,438 files | `15bd5568c69ce57f0df68365a2f600f26f7f6deea3a4b4de6fce52eb30ea5b33` |
| Root accepted archive, 1,448 files | `b446c4ac2f811ab8236f86df0867fe0d31f3850da4f9ea995463c4d4e448a765` |
| Root combined source, 1,481 files | `023f95d55a5ce62c75f5e7c786d4990e5ac6635b62a990cb820ca1d0e590d002` |
| New 93-file candidate manifest | `52bcdfcaa7fdc9d4424370a0d282f9f266df8b5b626374647c74372a767be79c` |
| Independently recomputed candidate content digest | `068fc2281212ba93e4ddcebf46f22101d034a8b5843fe7777da27d43372f8dcd` |
| Author combined source, 1,470 files | `02b06c4622c41daa65a0524ca9054974bbbdd2b07be9b2ac203069f15a767290` |
| New 42-artifact evidence freeze | `938ef98bd38159dd8cf601325393baa192e4f5bc403ac820e266cf045d36b04e` |
| New delivery | `d24b3ad6939e86f62f31b230ecd2357ec763e73b3f9b15cca284bc50486cea38` |
| Unchanged original probe-8 source | `142c6e87877634cfe43a0fd8165c2456bf3df581bf685e00ab5c565a0ca74050` |

`integrity-before.json`, corrected expanded verification and `integrity-after.json` retain the independent byte checks. All original **461 dependency, 45 redb and 82 Paper** file pins match, including exact pinned dependency inventories. Original design/archive, corrected author archive, historical round-3/4/5/6 candidate/evidence, prior round-3 review freeze, eight preserved intake inputs, new candidate locations, combined source and author evidence match. Round-3's separately preserved delivery also matches its old delivery pin; older delivery responses were read from the preserved historical evidence, not substituted with a current document. Historical failures, fixtures and freezes were left untouched.

The author basis was genuine main `55f362f6884ca3c4ab5e51cce545c3a5fb11d364`. The first erroneous `35e2556e…` baseline report was a `.dockerignore` hash and remains historical evidence. The corrected `15bd5568…a5b33` baseline is binding. Independently recomputed comparison agrees with Root's `basis-comparison.json`: exactly eleven documentation differences outside Storage; every non-documentation/non-Storage byte agrees. Comparing the author's combined manifest to Root's combined manifest adds a twelfth documentation difference, the separately added new delivery. This distinction was checked from files, not inferred from commit identifiers.

Builds used only review-owned `work/`, copied from Root's immutable combined tree, and review-owned `target/`. Every cargo invocation used `/home/jstrauss/.cargo/bin/cargo`, `--jobs 2`, `--locked`, and explicit separate target. The final complete focused rerun used review-owned `TMPDIR=.../tmp`. Final private-source inspection finds 1,480 of the 1,481 original files byte-identical; the only alteration is an explicitly verified appended unit-test include in private `block_validation/tests.rs`. Three reviewer integration sources are the only additional files. All outside-Storage bytes, including **324 accepted World files**, are identical to Root's immutable combined source. There was no shared World/Storage overlay, dependency change, implementation fix, Git operation or additional agent.

## R6-B2 closure and production review

**Historical finding:** medium severity, startup/liveness and cumulative-work acceptance gate. In round 6, `redb_store::block_validation::scan_with`'s scoped Journal branch retrieved, hashed and parsed the whole receipt Q per effect row, then parsed Q again without a charge. Probe 8 admitted a valid three-column Q=2,425,362 with 1,024 effects, but could not reopen at the original 1-GiB allowance; receipt read/hash work alone was 4,967,258,112 bytes. The required remedy was ordinary borrowed receipt/frame reuse per contiguous operation without changing limits or weakening proofs.

**Reviewed remedy:** `block_validation::scoped_journal` obtains one receipt using `owned::Read::owned`, then calls `block_durability::raw::frame_q` once. Its stack-local ordinary receipt guard and parsed borrowed Q remain alive while consuming exactly that operation's range. Both are dropped at the end of the while-body before acquiring the next operation. No self-referential owner/borrow structure, copied Q, historical receipt map, reset, discounted debit, new unsafe code, production global or capacity change conceals work. Q's vectors contain bounded parsed descriptors borrowing effect/image bytes; the large Q byte body is not copied.

The actual redb 2.6.3 path is `Table::get` → `BtreeMut::get` → `Btree::get_helper` → `AccessGuard::with_page`. The guard owns `PageImpl` with offset/length; `value()` borrows that page region. `PageImpl` retains `Arc<[u8]>` page bytes. Ordinary read guards use `OnDrop::None`. Storage's `RawGuard::as_ref` exposes this guard's value. This establishes safe actual page-backed ownership; it does not promise an RSS bound or exclusive physical-page allocation. The parsed frame cannot outlive its guard, and early `?` returns drop it normally.

`scoped_row` admits each raw key/value to raw and cumulative relation budgets, validates the eight-byte global sequence and row sequence, then the range loop checks scope, operation ID, ordinal, sequence, checked first+ordinal, and exact effect bytes. `receipt.first == count`, nonzero count, checked count+receipt.count, exact loop endpoint and globally contiguous keys prohibit truncated/extra ranges, interleaving and reuse of an old operation. `frame_q` binds receipt key, Q ID, scope, effect count, range grammar and digest before range consumption. Checked arithmetic and the MAX-sequence sentinel remain refusals. Inputs are shape-bounded before count/size arithmetic; new unchecked sums are sums of admitted bounded row lengths, not hostile unbounded allocations.

Every read remains charged by the budgeted reader; `frame_q` charges Q before parsing/hashing. For a nonempty operation with n chunk entries the branch's receipt contribution is **16 + R + Q = 87 + 9n + 2Q**, once per operation, plus every Journal key/value row. Probe 8 now contributes **4,850,838** receipt-work bytes instead of 4,967,258,112. This is only the changed branch contribution, not the whole opener requirement.

`scan_with` retains the complete catalog/metadata pass, current header/pair/last relations, every retained receipt's full endpoint proof, reverse exact receipt→Journal checks, counters/watermarks and disposable disk range nonoverlap index. Budget clones share one cumulative debit state. The bounded proof cache is still only an optimization, never authority. All writer/apply paths, production raw grammar and capacities are byte-identical to round 6; `engine.rs` and `raw.rs` changes add only cfg(test) observations. Existing `keep_cause`/`then_cleanup`, Fresh-open preflight, same-inode empty-file cleanup and existing-file refusal paths remain intact.

**Independent reproduction:** `final-logs/probe8-unchanged.log` records the original source unchanged, exit **101**, with `probe8 reopen cap=1073741824 -> None`. Its stale assertion expected `Some(ScanCapacity("bytes"))`; this is the explicitly required diagnostic failure, not a green test. Separate reviewer adaptation changes only the stale refusal/raised-limit expectation block and test name (`probe8-reviewer-adaptation.diff`). It retains the exact original fixture and config, public commit, receipt bytes, actual close/reopen, old-incarnation exact replay, raw pairs and views, all seventeen logical-table fingerprints and empty owned scratch. `final-logs/reviewer-adapted.log` passes. The candidate's separately adapted public probe also passes independently in `final-logs/probe8-adapted.log`. No limit increase, skipped reopen or waiver is used.

**Disposition:** R6-B2 and its uncharged second startup-Q-parse note are closed. No further remedy is required for this gate.

## Independent cumulative-work derivation and boundaries

`accounting.py` and `accounting.json` independently sum immutable admitted forms and the actual debit call graph. Neither candidate-reported requirements, a budget-counter measurement nor bisection is an input. Fixed full envelope F=49+24×4096×4=393,265; dense entity envelope D=49+16+16,384=16,449. These fixtures use at most four receipts, below the 128-entry full-proof cache bound.

For each operation, the model derives **Q=98+46n+Σbefore+Σafter+m(64+2E)** and **R=Q+71+9n**, where E is the per-side entity encoding for these particular fixtures. It accounts separately for counter reads (128), every nonmetadata raw key/value row, full/header current reads, descriptors, all receipt-table frame-Q work, additional relation Q parsing where table/proof order requires it, once-per-operation scoped Journal receipt work and 17 bytes per indexed range validation. Metadata is charged under the independent raw scan bound, not incorrectly added to relation work; physical scratch extent has its separate disk budget.

Full proof terms include scope descriptor and entry headers, current equal-outcome pairs, both endpoint-image decodes, exact final entity map encodings, reverse Journal read/re-encoding, canonical effect entity checks, old-entity encodings and new-entity application. In particular the dense proof includes 100,171,776 effect encoding/apply bytes and 66,993,136 reverse Journal bytes; counting only the changed Journal receipt term would miss most of the work.

| Fixture | Independently derived complete relation N | Fresh execution |
| --- | ---: | --- |
| Three-column small-effect/large-Q | **42,087,955** | Real close/reopen: N−1 refuses, N and N+1 pass |
| Multiple differently sized operations, including zero-effect image receipt | **61,761,009** | Real close/reopen: N−1 refuses, N and N+1 pass |
| Dense 1,019 effects, 16-KiB entities per side, Q=33,488,850 | **468,808,126** | Real close/reopen: N−1 refuses, N and N+1 pass |
| Reviewer-authored two-scope, five-operation fixture, two zero-effect receipts, nonempty ranges 2/6/4 | **14,549** | Real close/reopen: N−1 refuses, N and N+1 pass |

The first three are candidate tests freshly run by the reviewer, with an independently audited/derived oracle; they are not mislabeled as reviewer-authored tests. The fourth is the independent additional source `probes/reviewer_scope_boundaries.rs`, derived from no-payload immutable row grammar Q=144+64m, R=Q+80 and complete table/proof dispatch. All boundaries compare all seventeen logical tables and empty scratch after every success/refusal. The reviewer model explicitly handles table ordering: with no full rows, the receipt-table proof runs before the header path, so its additional Q parse is included. The five-operation fixture spans different scopes and has zero-effect receipts interspersed among the nonempty ranges.

A separate reviewer-authored direct-branch test establishes N=**3,984** for three nonempty operations plus a zero-effect receipt: 12×108 row bytes + Σ[96+2(144+64m)] for m=2/6/4. Caps 107/476/1,588/3,983 refuse with no retained guard; 3,984 and 3,985 pass at the exact full branch debit. This branch test complements, and does not substitute for, the full-opener boundary fixtures.

## Corruption, instrumentation and regressions

Fresh execution of complete `redb_store::block_validation::tests` covers the changed direction directly as well as real public reopen. Seventeen operation-transition cases exercise wrong scope/ID, orphan receipt, ordinal/sequence/effect disagreement, interleaving, old-operation reuse, missing start/truncated range/extra row, wrong first/count/overrun, zero-effect attribution, overlap and missing receipt. Forty-five broader cases retain catalog, metadata, row grammar, receipt/history, journal, counters and endpoint refusal coverage. Every real refused reopen preserves all seventeen logical tables and empties owned scratch.

The deliberately coherent receipt+Journal forgery updates the Q effect and correct SHA256 digest together. The changed relation-only branch **passes** it, as expected: matching bytes alone are not endpoint authority. The complete opener refuses it as `IncompleteDelta` because the forged terminal state contradicts the retained Keep endpoint. Thus the reverse/full receipt proof remains essential and active; there is no relation-only acceptance claim.

Instrumentation is local and entirely test-only: `JournalObservation` owns per-scan Rc/Cell observations, wraps the real retained guard, records acquisition, live/peak receipt count and sizes, and decrements on Drop. `Budget::frame_work` records actual parse/hash points in `raw::frame_q`, all under cfg(test). Production signatures omit the observation argument, wrapper and fields. The fresh candidate test proves **two reads, two parses, two hashes, peak one guard, zero afterward**, different receipt sizes, max receipt 42,667. This observation is scoped to the Journal direction, not a false claim of only two receipt reads in the entire opener.

The reviewer-authored additional guard/error test independently proves **three reads/parses/hashes, peak one, live zero** for three ranges. It checks early row-budget failure, failure during receipt fetch admission, later operation-budget failures, exact successful debit and a third-operation digest error after prior transitions. The public opener refuses the digest-corrupt database while preserving all seventeen tables and empty scratch. The >128 retained no-op receipt test, scratch physical extent/range checks, foreign replacement preservation, and primary-error-versus-cleanup-error test also pass freshly.

The whole candidate review preserves prior accepted authority/CAS/replay/no-op/history/atomicity/union/payload/endpoint/counter/bypass/uncertainty behavior. Fresh `player_block_contract` (3 tests) and `player_block_matrix` (9 tests) exercise actual redb close/reopen and old-incarnation replay; same-ID exact replay/collision namespaces; foreign authority and stale receipts; shared v1 autosave/transfer competition; same-chunk different-player and disjoint-union races; whole loser state; guard drift; image-CAS shared history; exact entity-only effects; hidden cells and complete union admission. Unchanged Memory/redb writer/apply, ownership, payload, Format6 common-row validators and Immediate commit/`CommitUncertain` paths were source-audited against the earlier accepted evidence. No new persistence or transaction boundary was introduced.

| Earlier finding | Status in this review |
| --- | --- |
| A1 whole-receipt runtime endpoint amplification | Prior fix preserved; touched-column proof and borrowed frame reuse unchanged |
| A2 failed Fresh-open retry obstruction | Prior fix preserved; scratch preflight/empty same-inode cleanup unchanged; cleanup tests fresh |
| H1 v1 refusal order, H3 complete race loser assertions, H7 lint rationale | Prior fixes preserved; writer unchanged, matrix freshly executed, documented lint allowances retained |
| H4 independent boundaries | Startup relation bytes now independently derived and challenged; older measured scratch/redb output bounds remain disclosed, not silently converted into proofs |
| B1 flat 16-KiB entity encoding charges | Round-6 actual encoding-length correction unchanged; dense complete-scan accounting includes real encodings |
| R2-H1 duplicate runtime Q parse | Round-6 runtime frame reuse unchanged; remaining startup Journal duplication corrected here; other deliberate parses remain charged |
| R2-H2 cleanup masking primary failure | Prior correction preserved and fresh dual-failure test passes |
| R2-H3 inode-check/path-unlink window | Historical low-severity residual unchanged; explicitly retained below |
| R6-B2 repeated startup Q work | Closed by source inspection, original probe-8 reopen, full-work boundaries and independent guard/error test |

## Executed checks versus audited author evidence

Final `run-final.py`, `final-runs.json` and original `final-logs/*.log` record **25 passing focused tests, no ignored tests**, plus the separately expected unchanged-probe exit 101. All focused tests were rebuilt/run from private immutable-basis sources. The full compiler log from the initial build is retained in `logs/`; final cached compilations are not presented as new full builds.

| Final command selection | Result |
| --- | --- |
| Original unchanged reviewer probe8 | Expected diagnostic exit 101; actual 1-GiB reopen succeeds |
| Candidate adapted probe8 | 1 pass |
| Candidate player_block_startup | 3 pass |
| Complete block_validation unit selection, including own guard test | 7 pass |
| Own two-scope full boundary | 1 pass |
| Own minimally adapted original probe8 | 1 pass |
| player_block_contract | 3 pass |
| player_block_matrix | 9 pass |

`audit_logs.py`, `author-log-audit.json` and `logs/author-audit.log` independently audit exact pinned author commands, results and logs against the frozen candidate: Debug Storage **360 passed**, Release Storage **360 passed**, neither ignored; consumers **884 passed, one Java test ignored**; workspace fmt, strict all-target Clippy, workspace build and strict Storage rustdoc exit zero. Expected compile-fail doctest diagnostics occur only inside the passing twelve-doctest blocks. These are **author-executed checks**, not newly reviewer-executed suites. Their source/dependency integrity is verified; no complete green matrix was repeated merely to create an identical second matrix.

Reviewer harness failures remain evidence and are not suppressed:

- The first comparison expected eleven differences against Root's combined tree and omitted the separately added delivery. `verify-initial.py` remains; the initial tool-call assertion is recorded in the review method deviations, rather than presented as an archived subprocess log. Corrected comparison proves eleven basis/twelve combined documentation paths.
- Expanded historical verification initially stripped repository prefixes against old round-3/4/5 copies whose layout is full-repository. `verify-expanded-initial.py`, `integrity-expanded-before.json` and its failure log remain. Corrected verification uses each nested crate root and separately checks round-3's preserved delivery; all files then match.
- The first reviewer boundary model assumed headers preceded receipts even with no full rows. Its N=13,621 omitted 928 bytes of legitimate additional Q parsing (400+528). The original source and failed compiler/test log remain. The corrected source-derived N=14,549 passes all three real reopen boundaries. The corrected total was not obtained by querying Storage's measured requirements.
- The first log audit treated expected compile-fail doctest errors as unexpected errors; the initial tool-call assertion is recorded in the review method deviations, with no separate original subprocess log. Corrected classification ties them to passing compile-fail doctests.
- An initial `startup_journal` filter matched zero tests; it supplies no coverage. The final actual block-validation selection runs seven tests.
- Initial test runs used the default system tempfile root, creating transient test databases outside the requested private directory; those temp products were automatically cleaned. This method deviation is disclosed. The complete final focused rerun explicitly uses the review-owned TMPDIR and final private tmp is empty. Persistent reviewer sources, logs and evidence are confined to the authorized review directory and requested new document.

None of these harness failures identifies a candidate defect; the original diagnostics are preserved alongside the corrected results.

## Preserved limits and acceptance boundary

No new finding requiring implementation changes is published. The following existing limitations remain explicit and are not waived or erased by this verdict:

- Arbitrary retained histories incur genuine cumulative startup raw/relation/cell/index costs. Once-per-operation reuse removes R6-B2's artificial effect multiplier; it cannot make one fixed local allowance admit every possible valid database. >128 cache eviction may repeat real full proofs. The independent exact models above apply to their specified fixtures, not arbitrary history.
- Runtime's hard **512-MiB byte / 16,777,216-cell** and **32-MiB snapshot-output** caps remain. Distinct large last receipts and effect-dense history can still refuse legitimate maximum shapes before any write, as disclosed and reviewed previously. Branch-only lower bounds are not complete runtime totals; the prior review's fuller eight-history 512-MiB+1,691 example remains valid, as does the documentation's explicitly narrower receipt-term lower bound +1,272.
- R2-H3 remains **low severity, disclosed residual** in `redb_store::uuid_migration::files::remove_owned_path`: symlink_metadata proves the regular file/inode, then removal is by path. A directory writer replacing that path between check and unlink could lose its replacement file. This review establishes ordinary mismatch/foreign-replacement preservation, not elimination of the adversarial race. Contract impact is cleanup ownership under concurrent directory manipulation; a future remedy would require an atomic identity-conditional deletion design/platform primitive. The code and earlier disclosure are unchanged; this was not the targeted blocking gate.
- Primary error preservation is checked for startup/refusal plus cleanup failure. Earlier logical uncertainty/Immediate transaction evidence remains accepted; no fresh process-kill or power-loss proof was performed. Logical-table preservation is not byte-identical redb-file or physical recovery acceptance.
- Format7 is still opt-in Storage work. Separate **DUR-03.3a migration** remains necessary before DUR-05. No legacy normalization, migration, App fallback or gameplay activation is accepted here.

There is no full Survival, Paper/client/plugin compatibility, performance/RSS, migration, process-kill/power-loss, or App/Sim activation claim. World generation remains the operator's priority. Only Root integrates.

## Result paths and freeze

All evidence paths below are relative to `.codex-tmp/dur03_3b-independent-review-r1/`:

- `integrity-before.json`, `integrity-expanded-corrected.json`, `integrity-after.json`, verification scripts and original failure logs: complete immutable input/history checks before and after.
- `private-source-after.json`, `private-integrity.py`: complete private inventory, exact authorized test additions, unchanged outside-Storage/World source, empty tmp.
- `delta.diff`, `delta.json`: actual whole candidate versus round-6 changes.
- `accounting.py`, `accounting.json`, `logs/accounting.log`: independent full-work derivation.
- `probes/`, both probe adaptation diffs, private `work/`: original and reviewer test sources, including failed initial forms.
- `run-final.py`, `final-runs.json`, `final-logs/`, initial `run*.py`, `runs*.json`, `logs/`: reproducible commands and every original compiler/test diagnostic.
- `author-log-audit.json`, `audit_logs.py`, `logs/author-audit.log`: pinned author-check audit, distinct from fresh execution.
- `assessment.json`: machine-readable verdict, executed counts, closure and retained limits.
- `review-evidence.freeze.json`: SHA256/size manifest of the evidence and private sources, with this review document separately pinned. Large `target/` products and empty tmp are excluded; compiler logs and all test sources are retained.
- `review-evidence.freeze.sha256`, `freeze-verification.json`, `result-pins.json`: final freeze digest, independent recheck and complete result pins. The manifest does not recursively include its own digest/recheck/locator files.

**Final disposition: CLEAN_RUNTIME_REVIEW for the pinned whole candidate; R6-B2 closed; no blocking remaining contract/runtime objection.**
