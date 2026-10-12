# DUR-03.3b — bounded Format7 startup receipt reuse

Root implementation assignment, 12 October 2026. Fix **R6-B2 before Storage
integration**. The round-3 independent verdict remains FINDINGS; there is no
temporary waiver, raised capacity, migration or App/Sim activation authority.
World generation remains the operator's priority and its two owners are separate.

## Readonly inputs and exclusive scope

Read AGENTS.md, CLAUDE.md, Storage INVARIANTS, architecture/storage,
ADR-0004/0007/0009 and PAPER_REFERENCE. Read the complete original
[DUR-03.3 contract](DUR-03_3_PLAYER_BLOCK_DURABILITY.md), original design/source
review and contract review, all three runtime reviews and the delivery, including
[round-3 findings](DUR-03_3_PLAYER_BLOCK_DURABILITY_RUNTIME_REVIEW_ROUND3.md).
Inspect the real redb 2.6.3 reader/guard implementation, not only an API signature.

You exclusively own `server/crates/ferrumc-storage/**`, the new
`docs/agent-tasks/DUR-03_3b_STARTUP_RECEIPT_REUSE_DELIVERY.md` and
`.codex-tmp/dur03_3b-runtime/**`. No other crate, Git, additional agents,
dependencies, lockfile, existing delivery/review/contract or historical evidence
may change. The shared Storage tree is the unaccepted round-6 candidate: audit it
before editing. Preserve its readonly copy and all previous logs/freezes.
Use the Root-supplied current-main archive and its baseline manifest under
`.codex-tmp/dur03_3b-root-base/` for private builds. Only Root integrates.

Root baseline pin correction after intake: the actual `base.freeze.json` SHA256
is `15bd5568c69ce57f0df68365a2f600f26f7f6deea3a4b4de6fce52eb30ea5b33`,
accepted main `55f362f6884ca3c4ab5e51cce545c3a5fb11d364`, 1,438 files freshly
reverified. Root's initial dispatch mistakenly reported the archive's
`.dockerignore` hash `35e2556e39d97afafa314e42c58ccfcc8791d69d655f403ffde8b8de91fa86f8`
as the manifest hash. No baseline bytes changed; the corrected pin is binding.
Preserve the first intake report and proceed on this exact supplied archive.

Binding input pins:

- Original contract SHA256 `2edfc588fd8d7e96753f6beb4bb7c2ad50a064e4c266681ab5ff94550c4342ab`.
- Round-6 manifest file SHA256 `d21db90ec56958c965c2a8fb288d21b82ad5af22c8bda9cd0d471cd894abd0d7`;
  91-file content digest `3c53838631108f43b76c689ebfd89dc2e3104901005ef1a9ae454d93039c6664`.
- Round-6 evidence freeze SHA256 `0335589a6f659d79a0750e53487d960356ba567138e6829f33ad8dc8f9b73e0c`.
- Round-3 evidence freeze SHA256 `88ab8eafd921d06a5f2de215a4fe010dd800a18e3e22cf89490c4638633406fd`;
  29 artifacts and separately pinned review document.
- Historical design/archive/dependency/redb/Paper pins remain binding. Current
  main is supplied separately; earlier author checks are not current acceptance.

## Defect and required implementation

`redb_store/block_validation.rs::scan_with` currently rereads and hashes a whole
receipt for every scoped Journal row, then parses Q a second time without charge.
For n chunks, Q request bytes and m effects, this branch alone charges
`m * (87 + 9*n + 2*Q)` bytes. A legitimate public commit with n=3, Q=2,425,362
and m=1024 cannot reopen with its original 1-GiB relation allowance: this branch
alone consumes 4,967,258,112 bytes. The independent probe 8 records the actual
same-limit failure, clean scratch and unchanged complete 17 logical tables.

During the scoped Journal pass retain at most **one current operation's receipt**
and reuse its charged `raw::frame_q` parse throughout that contiguous range.
Charge the actual receipt read/hash/parse once when entering a new operation.
Validate every row's scope, operation, ordinal, sequence and exact effect bytes;
retain global contiguous IDs, both relation directions, nonoverlapping receipt
ranges, complete receipt endpoint proof, counters, raw all-row scan and disposable
scratch index. Repeated old operations, interleaving, ordinal/range truncation,
overrun and mismatched effects must refuse. The cache is never authority.

Prefer retaining one borrowed redb row using safe lifetimes. Do not introduce
unsafe or a self-referential owner/borrow structure. If lifetime constraints
require a different bounded design, prove equivalent actual work elimination,
one-receipt memory bounds and all validation before freezing; report material
contract deviations to Root instead of silently changing authority. No unbounded
map, whole-Q copy merely for convenience, per-row reread, budget reset or accounting
discount while the work still executes. Remove the redundant uncharged Q parse.
Document key transitions, checked arithmetic, retained bytes and error precedence.

All existing request/image/player/entity/guard/effect/runtime/scan capacities and
persisted grammars stay unchanged. Distinct operation/history costs remain real
cumulative charges. A fixed scan allowance need not admit an arbitrarily large
database. This ticket removes amplification; it does not promise all shapes at
every configured limit. Round-6 B1, authority/CAS/replay/atomicity, primary error
preservation and the disclosed scratch inode/unlink window remain unchanged.

## Required genuine checks and delivery

1. Run unchanged reviewer probe 8 on the new candidate. Its original 1-GiB
   allowance must reopen successfully; verify actual whole-17-table equality,
   original receipt, old-incarnation exact replay, views and scratch cleanup.
   Preserve old failure evidence. Do not raise the probe's limits.
2. Derive independent exact scan byte accounting, then execute real reopen
   N-1/N/N+1 boundaries for small-effect large-Q, dense near-32-MiB entity effects
   and multiple operations. Keep fixtures within the immutable admitted shapes.
   Account all actual table, relation, endpoint and scratch work. Do not derive
   expected N from candidate output alone or pretend one-row work is all scan work.
3. Exercise genuinely corrupted operation/scope/ordinal/sequence/effect/range
   relations, interleaving and old-operation reuse; both directions and full
   endpoints still detect corruption. Refused opens preserve all logical rows
   and remove only owned scratch. Include zero-effect receipts and transitions
   between operations/scopes with different Q sizes.
4. Meaningfully demonstrate one-receipt retention and once-per-operation parse/
   hash work without production mutable globals. Test-only instrumentation may
   observe actual calls; source-only cost assertions are supplemental evidence.
5. Run complete Storage Debug/Release, strict all-target Clippy, fmt, warnings-as-
   errors rustdoc, required workspace/consumer checks on the exact supplied
   baseline plus whole candidate. Jobs 2, locked; use fresh private source/target.
   Preserve failed attempts and explicitly separate author from Root checks.

Before final checks announce a whole-Storage candidate manifest. Freeze a new
combined candidate and new evidence only after successful final checks; keep all
old freezes byte-identical. Report complete sources, delta, accounting proof,
limitations, commands/results and outside-file checks in the new delivery.
No runtime acceptance, process-kill/power-loss, performance, Paper gameplay,
Survival or App activation claim. DUR-03.3a still precedes DUR-05.
