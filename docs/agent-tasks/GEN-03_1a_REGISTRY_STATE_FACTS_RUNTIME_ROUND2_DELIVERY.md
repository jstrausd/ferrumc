# GEN-03.1a Round2 — concrete Registry candidate, open-attempt blocker

12 October 2026. **IMPLEMENTED CANDIDATE; G31A-OPEN-1 OPEN; NO RUNTIME
ACCEPTANCE.** The authorized checked-close capability is implemented and tested.
The complete Registry producer/API and its tests now exist. A subsequent actual
stdlib/callee audit found an additional native IO contract gap, reproduced below.
Passing crate tests do not close that gap or constitute publication/work/native
acceptance. The historical blocked Round1 delivery/evidence remain unchanged.

## Authority and scope

Original Rootbrief remains `c01af7e49fd0897b205b7d5aa37e11f6034ab412ec080d442d24494492fbd490`;
Round2 workorder remains `27a53cda29a264903661937dfcf1ebb4591d45a0bb3a1ec750a66644c5dee4ea`.
The selected dependency decision remains
`aef8edb79c0ca53b1dfb2999d26fb101a87201b71d1b5d05190ae69b348a7cdd`.
Original task/contracts/findings/responses/reviews remain binding. G31-R1-B2/M1
are preparation/tool dispositions, B1 is design-only; G31-R2-B1 and World Query53
composition remain outside this Registry split.

Evidence below uses `R = .codex-tmp/gen03_1a-registry-runtime-r2/`.
Writes are exclusively Registry, the necessary lockfile delta, this new document
and R. No Git, delegation, shared build, World/App/Flat write or consumer
activation occurred. The fresh private archive is genuine65c7, all1505 baseline
files/47 original Registry/324 accepted World. Its base freeze remains
`7917226aff23edc427743b73ee8451ae11b3db87faeb11fb2c26178ba1afb2ec`.
Only the full Registry and exact amended lockfile are overlaid for checks.

Root was notified before final checks of whole candidates, retained in
`candidate-before-checks{,-v2,-v3}.json`. The final source/lock candidate manifest
SHA256 is `4b7fa9690ff2962dbe4e0c9a6a374a4e860ea34651b0c7ad9931ccd96ff94f47`.
`candidate-after-checks.json` verifies every corresponding byte/mode/membership.
The final freeze and hashes are separate outer receipts to avoid a self-hash.

## Concrete implementation and authority

`build/state_facts_{codegen,sha,work,profile}.rs`, `build/checked_close.rs` and
build.rs implement whole blocks admission before the existing loader, followed
by the new phase after write/drop of the old block emission String/indexes.
Normalized metadata is borrowed, never cloned into another DOM. Exactly one
facts Vec, a u16[1105] insertion-sorted name index and3494-byte bitmap perform
full name/property/domain/forward-radix/default/ID closure for all27946 states.
True,false order and original enum/integer domain order remain intact; domain26,
seven pairs,192-byte property field,32-byte keys/values,64-byte names/fluids,
512-byte rows including LF,2MiB blocks and8MiB states are enforced. Unknown,
missing/extra/duplicate/noncanonical identities never inherit a default.

The scalar SHA implementation has bounded source loops and phase charges; no
library hash or sort substitutes for production work. Whole fixed input pins
precede parsing. Parser input length is borrowed from the completed hash, rather
than introducing a third input-stat action. The original directory must have
exactly six regular entries before payload reads; root/allowed/extra links,
FIFOs, sockets and nested directories refuse without opening their payloads.
Trusted immutable input/output ownership is required; hostile TOCTOU resistance
and crash durability are not claimed.

Unchanged genuine states/types/census/statuses/origins/trace and original
helper/output receipts are vendored with whole origin/byte/SHA pins in
`data/state-facts/provenance.json`. Production computational hash inputs are
blocks.json and states.tsv. Fixed descriptor/fluid/provenance literals are
checked against original receipts by producer tests; auxiliary TSVs are evidence,
not additional production hash inputs. The frozen independent fixture is
`tests/fixtures/original-state-facts.u16le`, SHA256
`95e27539aff80d384a7d9841454210da95ceb2ea8c56353d37b19c7d9cbb2701`.
It is copied from independent original evidence, never emitted by this producer.
Both actual joins and all runtime getters are compared against every genuine row.
No new Java run or algorithm-derived expected facts are claimed here.

`src/state_facts.rs` exports opaque `CheckedStateId`, fallible `TryFrom<u32>`,
bounded `UnknownStateId`, six closed `PredicateKind`s and immutable predicate,
air/motion/leaves/default/full-fluid/provenance/version getters. External
constructor, Default, infallible conversion and serde construction controls
actually fail compilation. This does not change existing World raw IDs. Local
fluid positions are private observed classes, never fluid network/bootstrap IDs.
Facts grant no collision/light/support/history or natural-world authority.

One bounded output record and4096-byte sink stream a225731-byte generated
file through exclusive part-create, flush, consuming checked close and rename.
Metadata emission reuses the fact record rather than repeatedly clearing a new
512-byte record. Old named bytes survive all demonstrated refusals. A cleanup
failure preserves the primary error and reports secondary errno; its part may
remain. Existing foreign parts are never deleted. Rename is the last fallible
computational/publication step; no sync/drop substitution is used.

## Checked close and exact dependency evidence

Only Unix build-only nix=0.31.3/default-features=false and Windows build-only
io-close=0.3.7/default-features=false were added. Cargo.toml explains the choice.
All pre-existing versions/checksums remain exact. `lock-delta.json/.patch` bind
six necessary additions: cfg_aliases0.2.2, nix0.31.3, io-close0.3.7, winapi0.3.9
and its two GNU0.4.0 import packages. Lock SHA256 is
`194c9e9d195c7318f710cbd57ee70e5b95bbf384b1fd9cc8784d27f2fe4e2a4b`.
No runtime dependency, unsafe code or lint exception was introduced.

`dependency-origin-source.json` inventories all source files of the selected
nine-package dependency closure, normal Cargo-cache origins/versions and actual
.crate hashes equal to lock checksums. Source audit covers nix close/Errno and
std File→FileDesc→OwnedFd consuming extraction/ManuallyDrop, plus Windows File
flush/extraction/OwnedHandle/CloseHandle. Pinned whole std callee sources from
compiler commit b940084d7eb6a299eb4bfeb8e34901bc051e7ac4 and fetch receipts are
in `std-callees/`; failed obsolete source paths are retained as404 diagnostics.
Unix io-close's EINTR-to-success implementation was read and is excluded.

Unix nix consumes ownership before one libc close. Every reported error refuses;
raw TLS errno is retained even when nix's enum would map an unknown code to
UnknownErrno. There is no retry or second Drop-close. `capability-final/` uses the
actual final adapter in fresh Debug/Release processes: real-file success, injected
libc EINTR4/EIO5/unknown31337 after actual successful close, and actual kernel
EBADF9 after test instrumentation invalidates the owned fd. Every refusal retains
old output; a reused /dev/null descriptor remains open with exactly one adapter
close. Injected libc returns are expressly distinct from kernel EBADF.

Windows uses exclusively File's safe CloseHandle implementation. In the pinned
std its preliminary File.flush is a no-op; handle extraction consumes ownership,
CloseHandle zero becomes last_os_error. Four adapter invocations imply four extra
zero-length flush calls/checks, zero additional native flush syscalls. The frozen
file-slot model counts whole close actions rather than Rust machine instructions;
if Root requires separate no-op bookkeeping units, the smallest Windows-only
proposal is +4, not silently incorporated into7391501569. Windows execution,
native size and CloseHandle faults remain unexecuted. Unsupported build hosts
have typed Native refusal rather than unchecked publication.

## G31A-OPEN-1 — native open attempts remain unbounded

Actual pinned std Unix `sys/fs/unix.rs::File::open_c` uses
`cvt_r(|| open64(...))`; the complete `sys/pal/unix/mod.rs::cvt_r` loops after
EINTR without an attempt counter. Thus `File::open` for hash/parser inputs and
`OpenOptions::open` for part creation can hide arbitrarily many native attempts
inside one charged high-level operation. Read/write/checked-close/rename/unlink
paths are one-shot; this separate open gap cannot be normalized away.

`open_retry_hook.c/.so` and `open-retry-boundary-final/receipt.json` exercise the actual
producer at libc open64 in both Debug/Release, with separate fresh old outputs:

| Injected EINTRs / terminal result | Actual part-open attempts | Outcome |
|---|---:|---|
|0 / real open success |1 |complete publication |
|1 / then real open success |2 |publication, first EINTR swallowed by std |
|17 / then injected EIO |18 |typed Entry/native5; old output unchanged |

These are injected libc results at the actual std/native boundary, not kernel
EINTR claims or a fake IO trait. The test's finite stimulus does not bound std's
loop; source plus the18-attempt counterexample disproves the strict finite/native
file-attempt interpretation. The existing recipe ceiling remains exactly
7391501569, but it does **not** establish a finite underlying open-attempt count.
This is a concrete remaining implementation blocker, not missing testgreen.

Smallest safe correction: separately authorize a one-shot consuming owned-open
primitive for the new Unix phase, for example nix's `fs` feature and safe open
returning OwnedFd→File, then audit its complete implementation, flags, path work,
ownership/error disposition, lock/features and repeat real producer checks.
The present exact nix close-only declaration has no fs feature. No such feature,
unsafe workaround, timeout/subprocess policy or silent contract relaxation was
added. Root must resolve this concrete scope/contract correction before acceptance;
this delivery asks no routine permission and does not self-accept the candidate.

## Work, literal static and native measurements

The unchanged14 conservative phase formulas sum to7391501569 u64. Whole permit
and per-phase precharges, phase omission refusals and N−1/N/N+1 tests are executed.
Fixed file slots total16 high-level operations; directory/output membership
bookkeeping is under the descriptor admission recipe. G31A-OPEN-1 limits what
that model establishes about native attempts. This is no CPU/time measurement.
Existing loader/emitter remain separate; their work is not claimed covered.

Actual DATA symbol is56504 bytes in both runtime artifacts. Facts/reference text
and alignment aggregate57432. `runtime-static-census-v2.json` follows actual
LLVM immutable-global references, including metadata text, error formatting and
panic source/locations: conservative57866 Debug/57465 Release bytes, below65536.
Optimized inlined getters' DATA is included explicitly even if no local function
body refers to it. Private source cap57431/57432/57433 compile controls refuse N−1
with E0080 and compile N/N+1. Their actual complete source/generated data is
retained in `native-controls-final-v2/`; older missing-README attempts are diagnostic.

| Actual additional inventory / measurement | Bytes or count |
|---|---:|
|Facts capacity27946×u16 |55892 |
|Borrowed Block Vec capacity2816 and header |180248 |
|Borrowed Property Vec capacity bytes /1794 properties |238392 |
|Borrowed String header capacity bytes /5911 values |164256 |
|All borrowed string capacities /2899 names |51871 |
|Complete normalized borrowed block inventory |634767 |
|Actual existing loader requested-heap peak /allocations |1542606 /17691 |
|Actual existing emitter requested-heap peak /allocations |567088 /194 |
|Old emitted source len/capacity/header |429841 /466944 /24 |
|New normal-path requested-heap peak /largest/allocation calls |59159 /55892 /31 |
|New OUT_DIR1024 requested-heap peak |61264 |
|Conservative complete source-derived new build admission |119380 |

Loader/emitter allocation windows are separate BEGIN/END measurements in fresh
processes, with131072 tracking slots after preserving an8192-slot exhausted
attempt. These are requested owned allocation bytes/capacities, not allocator
usable-size/RSS or total build memory. Existing item/mapping owners, borrowed
metadata and other build phases are additional stocks. No overlapping old source
String or second full-state identity index is hidden in the new phase.

Actual compiler `.stack_sizes` records include Debug generate closure24712,
Candidate::new11800, emit8344, parser closure5656, main4616, Sink::new4136 and SHA
compress1640; Release generate11704, emit5528, parse5400, main1080. Call-chain
selection includes main/helper/generate parents and compiler moves, not a sum of
mutually exclusive functions. The deepest selected direct emission chain is
43752 bytes before return-address/foreign-callee allowance.49152 is a conservative
whole-stack partition, not an extra arena or a measured whole-process peak.
6144 covers bounded path/callee heaps;8192 conservatively covers new readonly
producer constants (source census2584 plus derived diagnostics/location/alignment).
Total119380 is checked against131072 before candidate allocation, actual fact
capacity is rechecked, and N−1/N/N+1 admission controls execute. The directory
stream heap precedes live facts. The earlier1151-byte fluid-only constant partition
and112339 admission were superseded, not reported as complete final inventory.
Exact ELF attribution of build-only readonly constants and a complete independent
foreign-callee stack bound remain review/measurement gates; source partitions are
not falsely called literal measured simultaneous peaks. Windows is unexecuted.

`long-native-paths-final/` executes actual OUT_DIR1023/1024/1025 boundaries in both
modes: N−1/N publish, N+1 gives typed Native and preserves old bytes. Allocator
positive/failure probes are distinct. `runtime-{debug,release}-final-v2/` measures
first cold all-ID/getter/provenance/use and refused conversions in fresh processes:
zero allocation calls/peak/largest; independent4096 control has one call and
peak/largest4096. No boot/CPU/RSS/TPS/performance comparison follows.

## Executed checks, diagnostics and integrity

`final-checks-v3/receipt.json`: actual private locked/offline/jobs2 Registry
Debug/Release tests (72 tests plus14 rustdoc tests each), all-target Clippy
-D warnings, fmt, rustdoc -D warnings, accepted-baseline World/Proto lib consumer
checks all exit0. No pending shared World is used.
`producer-tests-{debug,release}-final-v2/`:22 actual build.rs tests each,13 new
producer tests plus9 existing build tests. Cargo does not run build.rs cfg(test);
the shipped reference runner selects exact Cargo JSON artifacts, compiles the
actual build.rs test source with build-only dependencies and forbids unsafe.
Compiler commands, edition and selected artifacts are retained transparently.

Tests include all-ID forward join/independent flags/default/fluid closure, strict
framing/canonical fields/unknown identities, missing/extra/duplicate rows, late
last-row ID/default/flag/fluid errors, independent private pins vs CatalogueDrift,
row511/512/513, keys/values31/32/33, names63/64/65, pairs6/7/8, properties191/192/193,
domains25/26/27,2MiB/8MiB regular-file length boundaries and actual hash admission
edges, scalar SHA padding/8MiB against independent sha2, every phase cap, exact
entry types, FIFO/socket/escaping/broken/allowed links and existing foreign parts.

`producer-faults-{debug,release}-final4/`:13 actual final producer processes each:
positive, short write, write EIO, close EINTR/EIO, rename EACCES, cleanup EACCES,
short hash/parser reads, hash/parser close EIO, allocator positive/failure. Old
output is byte-equal on every refusal; cleanup-eio alone retains its part and
reports secondary errno. IO faults are injected libc returns; real kernel EBADF
is separately recorded. File.flush itself is a pinned no-op, so no nonexistent
kernel flush failure is claimed; sink drain failures exercise actual writes.

All failures/superseded outputs remain: initial missing cfg_aliases offline
resolution, compile/Clippy attempts, exhausted metadata work permit test,
ambiguous newest-rlib selection, producer allocator phase/tracking failures,
Edition2024-vs-Cargo2021 fmt failure, incomplete initial readonly/static census
and missing README static controls. Final Cargo JSON dependency selection and
fresh names replace those routes. `native-compile-release-final2` is a rejected
mixed-serde artifact selection, not a Registry/runtime test failure. No historical
expected output/tool/source was repaired to make a result pass.

`baseline-after.json` matches the whole immutable archive (1833 lstat entries);
`private-archive-after.json` proves every private byte/membership outside Registry
and lock is that baseline. `blocked-after.json` matches the entire existing old
blocker tree against Round2 intake, including all1553 regular files/1885 entries
present then; these tree counts include Root's post-seal intake and differ from
its original1552/1884 sealed package census. Original blocked document and manifest
SHA pins remain exact. `historical.py after` invokes unchanged readonly original
verifiers and reproduces whole1817-entry review membership, author freezes,
4438 original inputs/reference/origin/historical pins. `authority-pins-after.json`
reverifies current original/continuation/blocker/decision hashes.

`outside-after.json`/`outside-changes.json` preserve complete shared outside
membership/bytes with explicit Git/cache exclusions. `outside-changes-classified.json`
separates10 concurrent external World/document additions/changes from an initial
outside inventory omission of the old blocked delivery. That omitted delivery
still has its exact independently pinned2a818... bytes; it is not Registry-worker
or external byte drift. Shared concurrent changes are not globally frozen claims
and were never imported into the private accepted build.

Final immutable source/lock/delivery/explicit evidence manifest and normal/-O
postfreeze receipts follow only after these successful checks and immutable
historical reverification. Volatile private build targets and four outer cyclic
freeze receipts are explicitly excluded; failed/superseded evidence is included.
Reproduction commands and artifact scopes are in the retained scripts/receipts;
use fresh output names, never overwrite old evidence.

Remaining Root gates: resolve G31A-OPEN-1 and any explicit Windows-only bookkeeping
interpretation; complete native foreign-callee/readonly attribution review;
execute Windows/other required host tests; obtain independent Registry runtime
review and Root integration/acceptance. World/Heightmap/Ore Query53 composition,
natural-world activation, Survival, whole FEATURES and Java/Paper plugins remain
separate work. This candidate provides no runtime self-acceptance or performance
claim and does not close those tickets.
