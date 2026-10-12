# DUR-03.3b — bounded Format7 startup receipt reuse

12 October 2026. Implementation delivery for Root's independent review and integration. R6-B2's scoped-Journal receipt amplification is removed. The original round-3 verdict remains FINDINGS until Root performs its acceptance. No capacity, persisted grammar, authority, replay/CAS/atomicity or activation change is included.

## Basis and preserved inputs

Accepted main is Root's supplied `55f362f6884ca3c4ab5e51cce545c3a5fb11d364` archive, 1,438 files. Its unchanged manifest SHA-256 is **`15bd5568c69ce57f0df68365a2f600f26f7f6deea3a4b4de6fce52eb30ea5b33`**. This is the binding correction in `.codex-tmp/orchestration/dur03_3b-baseline-correction.json`; the previous `35e2556e…` was the archive's `.dockerignore`, caused by Root reusing a reporting variable. No source/archive bytes changed. The first intake `.codex-tmp/dur03_3b-runtime/intake.json`, its script and the complete `round6-intake-readonly/` remain byte-identical. New work uses only `implementation-r1/`; no historical freeze or log was overwritten.

Before edits, all 91 shared Storage files matched the full round-6 manifest and readonly copy. Binding contract `2edfc588fd8d7e96753f6beb4bb7c2ad50a064e4c266681ab5ff94550c4342ab`, round-6 manifest file `d21db90ec56958c965c2a8fb288d21b82ad5af22c8bda9cd0d471cd894abd0d7`, its content digest `3c53838631108f43b76c689ebfd89dc2e3104901005ef1a9ae454d93039c6664`, round-6 evidence `0335589a6f659d79a0750e53487d960356ba567138e6829f33ad8dc8f9b73e0c`, and round-3 evidence `88ab8eafd921d06a5f2de215a4fe010dd800a18e3e22cf89490c4638633406fd` were verified. All 38 round-6 evidence artifacts, 29 round-3 artifacts and its separately pinned review, 16 original design artifacts, original 1,224-file archive, 461 dependency files, 45 redb files and 82 Paper files were verified too. `audit.json` and `historical-after.json` record complete inventories and mismatches (none). The original design/source review, contract review, all runtime reviews and complete prior delivery were read with required repository/Storage instructions and ADRs/reference.

No Git or other agents were used; no dependency or lockfile was changed from round 6. Only Storage, this new delivery and `.codex-tmp/dur03_3b-runtime/**` were written. World/Sim WIP was never used as a check dependency. All check builds use the supplied accepted-main archive plus the entire candidate Storage tree. Toolchain: rustc 1.99.0 `(b940084d7 2026-09-28)`, cargo 1.99.0; `/home/jstrauss/.cargo/bin/cargo`, jobs 2, locked.

## Complete candidate and delta

Whole Storage was announced before final checks: **93 files**, content digest **`068fc2281212ba93e4ddcebf46f22101d034a8b5843fe7777da27d43372f8dcd`**, manifest file SHA-256 **`52bcdfcaa7fdc9d4424370a0d282f9f266df8b5b626374647c74372a767be79c`**. `candidate-manifest-r1.json` lists every file, byte length and SHA-256, including unchanged files. `candidate-src-r1/` is its readonly full source copy; the shared Storage tree is identical. The fresh combined private source has 1,470 files. Final source/target names are `final-source-r1/` and previously absent `final-target-r1/`.

Changed from round 6: `INVARIANTS.md`, `README.md`, `src/redb_store/block_validation.rs`, its `tests.rs`, and test instrumentation in `src/block_durability/{engine,raw}.rs`. Added `tests/player_block_startup.rs` and `tests/player_block_startup_probe8.rs`; nothing removed. `delta.json` and `round6-delta.patch` provide the exact changes. Writer/commit/apply modules and immutable shape/persistence capacities are unchanged.

## Implementation, lifetime and retained bytes

`scoped_journal` consumes a whole contiguous operation range in an inner loop. Its actual owning receipt is read through the existing budgeted reader once, and `raw::frame_q` returns the one charged parsed Q plus borrowed frame. The redb guard is an ordinary local owner; Frame/Q borrow it on the stack. The guard and both parsed values go out of scope before the next operation's guard is acquired. There is no self-referential owner/borrow structure, unsafe code, full-Q copy, historical receipt map, per-row receipt reread or budget reset.

The genuine installed redb 2.6.3 implementation was inspected: `Table::get` (`src/table.rs:268`) delegates through `BtreeMut::get` (`tree_store/btree.rs:505`) into the read tree. `Btree::get/get_helper` (`:770–797`) finds the leaf value range and returns `AccessGuard::with_page(page, start..end)`. `AccessGuard` in `tree_store/btree_base.rs:162–247` owns its PageImpl plus offset/length; `value()` borrows the page's slice, and ordinary read guards use `OnDrop::None`. This is actual page-backed ownership, not an API-signature inference. Storage's RawGuard lends that row with `AsRef<[u8]>`. No writer mutates a receipt while it is borrowed in this scan.

At most one receipt value is retained by the Journal pass, bounded by Q≤32 MiB and the FCBR frame hard bound Q+143 bytes (actual overhead 71+9n, n≤8). Frame metadata has ≤8 outcome stamps; Q metadata has ≤8 entries, ≤4096 guards and ≤1024 effects. Entries/effects borrow every image/entity payload; metadata does not duplicate Q. One bounded current Journal row is held as well. redb backing-page/allocator/cache overhead is separate; this is not an RSS guarantee. Existing full receipt proof caches and main/index cache capacities remain unchanged.

On entering a range, the first actual Journal row is admitted to raw and relation budgets, its global key/sequence checked, its owning receipt fetched and frame/digest checked. The receipt must start at the current contiguous ID and have a nonempty range. Checked `first+count` establishes its exclusive end. Every row independently verifies scope, operation ID, ordinal, sequence, checked `first+ordinal` and exact effect bytes against Q. A missing next row refuses truncation; every extra row must belong to a new range starting at the next contiguous ID. This rejects overrun, interleaving and old-operation reuse without retaining old IDs. A zero-effect receipt has no Journal range and cannot own a row.

The cache never replaces authority: the preceding FCBR table pass still performs the whole receipt proof and reverse exact-ordinal relation, Chunk/Player endpoint/history relations remain intact, the raw scan still covers all 17 tables, counters/watermarks are still independently summed, and nonoverlap still uses the same disposable bounded disk index.

Error precedence: metadata/catalog and earlier table proofs still run first. In this branch raw/relation admission precedes row decoding; key/sequence precedes receipt access; orphan/FCBR grammar/digest precedes range entry and per-row agreement; following-row admission precedes agreement, and missing rows report range truncation. A malformed relation can be rejected by the earlier reverse direction before this branch; direct branch tests cover that distinction. Error classes and primary-error-versus-cleanup behavior stay unchanged. Reducing actual work can expose a later genuine error where the old code exhausted its budget first. Existing scratch inode-check/path-unlink window remains disclosed and unchanged.

## Independent accounting and genuine reopen boundaries

For a nonempty receipt with n entries, Q request bytes and R=Q+71+9n:

- old Journal receipt work: `m*(16+R+Q) = m*(87+9n+2Q)`;
- new Journal receipt work: `16+R+Q = 87+9n+2Q`, once for that operation;
- every Journal key/value still costs its actual row length. The receipt-table frame, endpoint proofs, histories, raw scan and scratch validation are additional cumulative work.

The complete independent byte model in `player_block_startup.rs` reads the actual whole table inventory, uses fixed contract header/layout offsets and known constructed fixture histories, and never reads a candidate-reported budget. It accounts 128 counter-read bytes; all non-meta raw key/value rows; every Full/Header identity, pair/history read and real pair copy; each last receipt read/hash; every full endpoint image decode and canonical entity encoding/apply decode; every FCBR-table frame and second relation parse when actually needed; reverse Journal read/reencoding; once-per-nonempty-operation Journal receipt work; and 17 bytes for each scratch range-index validation row. Metadata rows also count toward the separately bounded raw scan. Scratch creation/insertion/abort physical extents remain governed by the unchanged actual disk backend budget; the 17-byte term is the logical range-row validation charge, not a physical I/O or allocation estimate. These fixtures have fewer than 128 receipts and exercise the unchanged bounded proof-cache regime.

| Actual exclusive reopen fixture | Exact relation N | N−1 | N | N+1 |
| --- | ---: | --- | --- | --- |
| Three noisy full columns, two small cycle effects, large Q | 42,087,955 | `ScanCapacity("bytes")` | opens | opens |
| Three differently sized operation ranges (3/1/3-column Q), retained zero-effect image receipt | 61,761,009 | same | opens | opens |
| One column, 1019 alternating 16-KiB entity effects, Q=33,488,850 (near32 MiB) | 468,808,126 | same | opens | opens |

All cases use real public commits within the immutable admitted shapes. Each open/refusal compares every logical row of all 17 tables before and after and checks an empty dedicated scratch directory. Setup uses 1 GiB; the final exact-boundary opens change only the explicitly local relation allowance. Distinct operation costs increase the full-scan total, and no candidate output is used to choose N.

## Reviewer probe 8 and honest old-oracle result

`probe8-unchanged.rs` is byte-identical to the round-3 reviewer source, SHA-256 `142c6e87877634cfe43a0fd8165c2456bf3df581bf685e00ab5c565a0ca74050`. It was executed on the new candidate without changing its original limits. Its old assertion of a 1-GiB refusal fails because the real opener now succeeds (`reopen cap=1073741824 -> None`); this exit101 log is preserved as `logs/probe8-unchanged.log`. The old failure evidence and its freeze remain unchanged. It is not reported as a green test.

The separately named regression `player_block_startup_probe8.rs` changes only the stale refusal/recovery expectation into reopen at the *original* configuration and the test name; original fixture, public commit, lookup/replay, byte formulas, views and whole-table comparison remain. Q=2,425,362; R=2,425,460; m=1024; old branch 4,967,258,112 bytes. New branch receipt work is 4,850,838 bytes. The original 1-GiB reopen succeeds, the original receipt is byte-identical, the committed old-incarnation request replays, raw pairs/materialized views match, all17 tables remain identical, and scratch is empty. No probe limit was raised.

## Corruption, transitions and measured work

The new 17-case corruption test exercises real scope/operation/ordinal/sequence/effect alterations, orphan/deleted receipts, interleaving, old-operation reuse, absent range start, missing last row, extra row/ordinal, incorrect range first/count, overlap and a Journal row attributed to a zero-effect receipt. It runs the changed Journal direction itself under actual redb readers as well as the full opener. Refused opens preserve all17 logical tables and clean only owned scratch.

A consistently forged effect in both receipt Q and Journal, with correctly recomputed SHA-256, intentionally passes the Journal relation-only direction, then refuses the full open with `IncompleteDelta`: the complete endpoint proof remains essential authority. The existing 45-case catalog/history/counter/grammar corruption test and range-index/foreign-replacement/primary-error tests run unchanged in the full suite. The old round-6 dense runtime residual test also stays byte-identical: it constructs fresh stores and does not cover reopen; its old per-row-startup-cost comment describes the superseded round-6 behavior. The new independent dense startup test above performs the actual close/reopens on this candidate.

Local test-only instrumentation wraps actual redb receipt guards and observes their Drop; frame instrumentation counts the executed Q parser and SHA digest calls. Two operations with four/two effects, different Q sizes and different scopes produce **2 receipt reads, 2 frame parses, 2 hashes, peak1 retained receipt, zero live guards afterward, max retained receipt value42,667 bytes**. The complete cross-scope database also reopens unchanged. Every tested failure drops all acquired guards and retains at most one. No production globals are used; production builds omit the counters and guard wrapper. Source cost assertions supplement these real observations.

## Final author checks

| Final author check | Result | Seconds |
| --- | --- | ---: |
| Workspace fmt check | exit0 | 2.982 |
| Complete Storage Debug | exit0; 360 passed, 0 failed, 0 ignored in 20 result blocks | 1359.878 |
| Complete Storage Release | exit0; 360 passed, 0 failed, 0 ignored in 20 result blocks | 490.608 |
| Workspace all-target strict Clippy (`-D warnings`) | exit0 | 67.964 |
| Storage rustdoc (`RUSTDOCFLAGS="-D warnings"`, no deps) | exit0 | 10.538 |
| Workspace all-target build | exit0 | 231.622 |
| Sim/App/Testkit/Bench consumers | exit0; 884 passed, 0 failed, 1 ignored in 63 result blocks | 207.22 |

Commands/results are in `final_checks.py`, `final-results-r1.json`, `final-run-r1.log` and `final-logs-r1/`. Every compilation/check uses jobs2 and locked; fmt has no dependency resolution/build. Debug and Release exercise the complete Storage suite, not just the new tests. These are this implementation author's results on the exact supplied baseline plus candidate, not Root acceptance or earlier-author/current-main equivalence.

Failed development attempts are retained: initial private copy inherited readonly modes and refused deletion before overlay (no test ran); a first boundary test had incorrect ID getter/map iterator syntax; an observation fixture incorrectly repeated an unchanged gameplay effect, then tried to initialize a new scope after nonempty state; strict Clippy caught test-only style issues. These fixture/style mistakes were corrected before the manifest announcement. `logs/` preserves every failed test/compile/check and the successful follow-ups. The old probe's exit101 is the stale refusal assertion described above, not an unexplained candidate failure.

Every one of the 1,470 combined source files stayed byte-identical before/after final checks. Whole Storage equals the announced manifest in the shared tree, readonly candidate and private check copy. Workspace Cargo.toml/Cargo.lock and the round-6 Storage Cargo.toml are unchanged. Source comparisons do not equate concurrent shared World/Sim WIP with accepted inputs. The only additional private-source files created by consumer checks are separately inventoried in `source-after.json`: `.codex-tmp/p57-plugin-testhost-3529018-artifacts/libplugin_testhost_fixture.so` (8459304 bytes), `.codex-tmp/p57-plugin-testhost-3529018-artifacts/librenamed_fixture.so` (8459304 bytes). They are check products, not candidate source files.

The new clean readonly `combined-candidate-r1/` contains exactly these 1,470 checked source files, excluding reported test products. `combined.freeze-r1.json` freezes the complete combined inventory after successful final checks; `evidence.freeze-r1.json` freezes the complete new reports/scripts/logs and this delivery. Historical freezes and inputs remain unchanged; `historical-final.json` records the final complete recheck. The freeze is author evidence, not a Root runtime verdict.

## Limits and Root boundary

No material contract deviation or unresolved implementation contract issue was found. Root owns independent review and integration. The full database still needs cumulative raw, relation-byte/cell and physical scratch allowance; distinct histories remain charged and an arbitrary database is not guaranteed to reopen under any fixed allowance. Round-6 B1's exact entity accounting and runtime hard-byte residuals are unchanged. Existing logical-preservation versus redb physical recovery distinction and scratch unlink window remain unchanged.

No runtime acceptance, performance/RSS, process-kill/power-loss, live Paper/client gameplay, full Survival, migration or App/Sim activation claim is made. DUR-03.3a remains required before DUR-05. World carver/ore work remains with its separate owners.
