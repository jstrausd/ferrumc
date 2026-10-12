# GEN-03.1a — Registry checked-close capability blocker

12 October 2026. **BLOCKED BEFORE IMPLEMENTATION; NO REGISTRY CANDIDATE.**
The final Root workorder is pinned at
`c01af7e49fd0897b205b7d5aa37e11f6034ab412ec080d442d24494492fbd490`.
This reports the exact platform/Rust capability gap requested by §3 of that
workorder. It does not replace the frozen Revision2 contract or reviews.

## G31A-CLOSE-1 — checked close unavailable in the authorized safe builder

Revision2 §2 reserves checked hash-input, parser-input, partfile and failure
close operations. Section3 requires checked flush/close before the sole rename,
and real close-refusal evidence preserving the previously published output.
Workorder §3 explicitly requires reporting a platform/Rust close gap instead of
silently weakening the contract or treating injected wrappers as real IO evidence.

The installed compiler is Rust1.99.0, commit
`b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`, host
`x86_64-unknown-linux-gnu`. Actual Debug and optimized Release compile controls
produce E0599 for `File::close()` and E0308 when `drop(File)` is returned as
`io::Result<()>`. These are expected capability refusals, retained verbatim in
`.codex-tmp/gen03_1a-registry-runtime/capability/` with complete commands and
exit codes in `receipt.json`.

Whole upstream Rust sources from that exact compiler commit are retained with
origin URLs, bytecounts and SHA256 in `upstream-source-receipt.json`:
`fs.rs` documents ignored close errors at lines58–60;
`os/fd/owned.rs::OwnedFd::drop`, lines200–225, discards `libc::close`'s result.
`File::sync_all` calls fsync while the descriptor remains open; it cannot report
the subsequent close return. `File::flush` is currently an unbuffered no-op on
Unix/Windows. A bounded custom output buffer could implement the required flush,
but it cannot recover the discarded close result.

Registry's allowed build dependencies are serde/serde_json/sha2. They expose no
checked OS close capability. Workspace `unsafe_code = "forbid"` applies through
Registry's inherited lints. New dependencies/lockfile changes are prohibited by
the workorder. A direct FFI checked-close call consequently also needs an explicit
safety/lint exception. A fake `close` method returning `Ok(())`, an injected trait
error, BufWriter unwrapping or a sync-then-drop substitution would not satisfy the
existing checked-close and actual-refusal obligations.

**Smallest correction preserving the checked-close contract:** Root must
authorize a build-only safe checked-close dependency and the corresponding
`server/Cargo.lock` update, then pin/review that concrete adapter before use.
Its API must consume descriptor ownership and return the actual one-shot OS
close result. The platform contract must specify descriptor disposition on
EINTR/error without retries or a second Drop-close. This requires no runtime
dependency, World/App change, second output file or relaxed byte/work cap.
The dependency choice/version and its platform implementation are not claimed
verified here. An alternative direct FFI solution needs a separately authorized
build-only unsafe/lint/safety-document exception and is not authorized today.

If Root instead chooses a safe-std-only sync/drop publisher, that is an explicit
publication-contract amendment: it must replace checked-close/close-refusal
requirements and account for the added fsync operation and errors. It cannot be
presented as the unchanged Revision2 publisher. No such amendment was made.

## Executed evidence and unchanged authority

Two fresh native capability-positive processes, Debug and Release, successfully
executed exclusive part-create, single complete write, flush, sync_all, drop and
rename using only safe std. Their before/after payloads and literal outputs are
retained. These prove that exact success sequence only. No actual close failure,
injected close wrapper, full GEN-03.1a publisher or production IO acceptance is
claimed. Four compile-refusal controls cover absent close and unit-return drop
in both builds. Complete native compiler identity is recorded.

The genuine65c7 archive was verified byte-for-byte and with exact regular-file
membership:1505 files, including47 Registry and324 World,261375921 bytes.
`base.freeze.json` remains
`7917226aff23edc427743b73ee8451ae11b3db87faeb11fb2c26178ba1afb2ec`.
A new private exact archive copy exists under the authorized evidence scope.
It was never overlaid with shared World and never built. Registry remains the
unchanged47-file/2913532-byte source tree. No Git, agents, production, dependency,
lockfile, shared-build, World, Flat or App write occurred.

`outside-before.json` inventories the complete shared project source tree outside
Registry and this authorized delivery:5587 regular files/749664436 bytes,
7086 lstat entries. Explicit exclusions are `.git`, `.codex-tmp`, `server/target`,
`target`, `node_modules`; this is not a claim to inventory unrelated mutable
child evidence or Git/build-cache internals. The separately invoked unchanged
historical verifiers rehash the whole Revision2 author/review memberships, all
4438 original input pins,175 reference pins,20 origin copies and historical
GEN-03.1/GEN-06.1 freezes. Links/FIFOs/sockets are inventoried as entry metadata
and never read as payloads. Reviewfreeze remains
`bdea7cfd73cb7bbf983bd60656a13578deaeb01e8ebb8db3efe97ff4699b7300`;
whole review inventory has1817 entries/1404 regular files/1116541431 bytes.
Final before/after receipts and any concurrent outside changes are reported in
the evidence area, rather than being attributed to this worker.

Final archive, private-copy and Registry before/after comparisons pass exactly.
Shared source comparison records three concurrent documentation changes:
`docs/IMPLEMENTATION_BOARD.md`,
`docs/agent-tasks/GEN-05_11_OVERWORLD_CARVERS_RUNTIME_REVIEW.md` and new
`docs/agent-tasks/GEN-06_3_ORE_ALGORITHM_REVISION3_TASK.md`. Their entire before/
after hashes/types/lengths are preserved in `outside-changes.json`; this worker
did not write them or import their authority. No outside runtime/dependency/
workflow byte change was observed. Historical before/after verification is
recorded separately and must pass before the blocker evidence manifest closes.

The current full workorder copy and pin are in `workorder-pins.json`.
`runtime-task-dispatch-provenance.md` reconstructs the earlier complete on-disk
dispatch workorder from its full prior read and Root's exact final-ending delta;
its SHA256 is `f3726d1323113f5154950b2f9c24d2790255739ae168e52b2ea8f15eee949786`.
It is explicitly provenance, not the final Rootbrief or a newly authoritative
historical original. All frozen review documents remain untouched.

## Remaining implementation and acceptance gates

This capability gate was checked before writing a partial producer. The full
required original-method/callee/base/feature semantic audit remains an
implementation prerequisite; integrity hashing is not a claim to that audit.
The actual parser, metered hasher/join/stream, static facts, CheckedStateId and
runtime API have not been implemented or measured. Their malformed/all-ID,
native/allocation/N-boundary and actual publication-fault tests remain required,
as do private archive-overlay Registry Debug/Release tests, Clippy, fmt, rustdoc
and any affected consumer checks. None of these absent checks is reported green.
There is no complete immutable candidate notification or runtime-candidate
freeze; the new manifest records blocker evidence only.

G31-R1-B2/M1 remain preparation/tool dispositions, G31-R1-B1 remains design
disposition and G31-R2-B1 remains open for the future World/Ore adapter. The
accepted Registry65536/131072-byte, domain26 and7391501569-u64 limits remain
unchanged and unmeasured. Natural-world activation, Survival and whole FEATURES
remain outside this ticket. Java/Paper plugins are not implemented by this
producer. No runtime acceptance, RSS/TPS or performance improvement follows
from this delivery.
