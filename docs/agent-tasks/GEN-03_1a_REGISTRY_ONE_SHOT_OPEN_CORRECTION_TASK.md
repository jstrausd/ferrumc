# GEN-03.1a Round3 — one-shot owned Unix open correction

Root implementation correction, 12 October 2026. Exclusive Registry writer.
Round2 is a concrete frozen implementation with G31A-OPEN-1, not accepted
runtime. Fix the demonstrated native open retry without relaxing the original
producer/publication/work/native contract. Root owns integration; World/Ore
Query53 and natural-world activation remain separate.

## Binding original brief, findings and responses

Read complete original `GEN-03_1a_REGISTRY_STATE_FACTS_RUNTIME_TASK.md`, original
GEN-03.1 contracts/findings/responses/reviews, Round1 blocked delivery,
`GEN-03_1a_REGISTRY_CHECKED_CLOSE_RESUME_TASK.md` and the complete Round2 delivery
`GEN-03_1a_REGISTRY_STATE_FACTS_RUNTIME_ROUND2_DELIVERY.md`. Preserve every original
parser/join/hash/meter/static/native/IO/refusal/publication/test obligation and
original27946-state/1105-block facts. Read AGENTS/CLAUDE/Registry invariants,
relevant architecture/reference/source and selected dependency/callee closure.

G31A-CLOSE-1: unchecked std Drop could not report close errors. Root authorized
only Unix build-only nix0.31.3 and Windows io-close0.3.7 File/CloseHandle, no
Unix io-close, no local unsafe. Round2 implements consuming one-shot close and
executes real-file success, injected libc errors and actual kernel EBADF in both
profiles. Preserve that evidence and complete source behavior; do not call
injected returns kernel faults. Windows remains unexecuted.

G31A-OPEN-1: pinned std Unix File::open_c wraps open64 in unbounded cvt_r EINTR
retry. Both actual final producer profiles observed17 injected EINTRs followed
by EIO:18 native attempts inside one high-level open slot. Earlier positive and
one-EINTR-then-success observations expose swallowed EINTR. Thus green crate
tests do not close finite native-attempt authority. Existing File::open hash/
parser inputs and exclusive OpenOptions part-create in the new phase are affected.
The original existing loader/emitter remain explicitly separate phases.

Earlier G31-R1-B1/B2/M1 and G31-R2-B1 dispositions remain visible. This task does
not accept a World adapter, Query53 composition, extra public caps or Java/Paper
runtime. Native readonly/foreign-callee and Windows gates are not silently
closed by this correction.

## Exact Root source decision

Root authorizes only adding `features = ["fs"]` to the existing Unix build-only
`nix = { version = "=0.31.3", default-features = false }`. Use safe
`nix::fcntl::open` once, transferring successful OwnedFd into File once. Every
reported native error, including EINTR, refuses; no retry, fallback std open,
second Drop-close, runtime dependency, local unsafe or new dependency. Preserve
the authorized checked-close adapter and target Windows dependency unchanged.

Full source decision/pins:
`.codex-tmp/orchestration/registry-one-shot-open-dependency-decision.json`.
SHA256 `3db7170bb4e05d6c8a9c20c676f8201935f9ba6bde0ea279092bcc6aba3f9805`.
The Root-reviewed nix open method invokes libc::open exactly once, translates
Errno and returns OwnedFd; fs is an empty feature with no additional feature
dependencies. Verify selected actual compiler artifacts/features and complete
origin/lock/checksum/callee evidence yourself. Necessary lockfile resolution only;
do not update unrelated package versions/checksums. Record unchanged lock bytes
if enabling fs needs no lock change.

Inputs retain read-only+CLOEXEC semantics; part creation retains write-only,
create+exclusive+CLOEXEC, mode0666 filtered by process umask. No truncate/reuse or
deletion of foreign parts. Preserve the existing trusted immutable ownership
contract and exact regular-entry admission; no new TOCTOU-security claim.

Native/path admission includes actual nix NixPath conversion: Path forwards to
bytes, stack conversion below1024, allocating CString at length>=1024. An
explicit bounded CStr route is allowed if its copy/validation/storage is counted.
Embedded NUL/path refusal is distinct from returned syscall errno; never read
stale TLS errno for a pre-syscall validation error. Preserve unknown native errno
correctly, exact ownership and primary/cleanup error precedence.

## Immutable baseline, correction subject and scope

Original genuine accepted65c7 archive1505/47Registry/324World remains
`.codex-tmp/gen03_1a-registry-root-base/archive`, base freeze
`7917226aff23edc427743b73ee8451ae11b3db87faeb11fb2c26178ba1afb2ec`.
Round2 original freeze SHA
`870034ff8bd903b284b87cbb510c3490a4007412d710beadd9af0da4bfe29189`,
3745entries/3075regular/1352274403bytes; doc SHA
`79729e7a33417496cc5dd3ce08614668c302d4de2cb8abad8b0216eee6083793`;
lock SHA `194c9e9d195c7318f710cbd57ee70e5b95bbf384b1fd9cc8784d27f2fe4e2a4b`.
Root independently verified every declared byte/type/mode/membership and six
links without following payloads, plus exact native18-attempt receipts.
Receipt `.codex-tmp/orchestration/gen03_1a-registry-round2-intake.json`.

Before allowing shared source replacement Root preserved complete Round2
Registry66 + lock + delivery68 regular files readonly under
`.codex-tmp/gen03_1a-registry-r3-root-subject/archive/`;
`source.freeze.json` SHA
`0c720c432ece838a636be1896babf43f970fd3a85b1e4abf7bc29aacc8ee8701`.
Original Round2 freeze references shared source paths: after authorized Round3
replacement verify these historical paths through this explicit readonly Root
snapshot. Verify all remaining original Round2 evidence at unchanged original
paths. Never edit the original freeze or pretend old live source is unchanged.

Build a fresh private copy of the original accepted archive, overlay whole
snapshot Registry and exact Round2 lock, then apply correction. This is an
explicit unaccepted correction subject; never build shared mutable World/lock.
Use Cargo `/home/jstrauss/.cargo/bin/cargo`, locked/offline/jobs2, private target.
Record complete baseline/outside bytes and source/lock overlay manifests.

Write only `server/crates/ferrumc-registry/**`, necessary `server/Cargo.lock`, new
`docs/agent-tasks/GEN-03_1a_REGISTRY_ONE_SHOT_OPEN_CORRECTION_DELIVERY.md` and
`.codex-tmp/gen03_1a-registry-runtime-r3/**`. No Git, agents, other crates/docs,
old evidence mutation, World/Flat/App changes or activation.

## Required meaningful implementation and checks

1. Replace every affected new-phase Unix open/create with the checked one-shot
   primitive. Read full success/refusal/cleanup source paths and complete selected
   dependency/stdlib ownership/callees. Preserve generated bytes, all original
   facts, flags/defaults/fluid/provenance and opaque checked-ID API.
2. Execute actual safe adapter and actual producer boundary probes in fresh
   Debug/Release processes. Input hash open, states hash open, parser reopen and
   output exclusive part open must all show one attempt on EINTR/EIO/unknown
   native errors and positive success. Adapt hooks to actual libc open/open64
   symbol; do not reuse std-only open64 injection and claim the nix call was hit.
   Positive controls prove actual symbol/descriptor ownership. Refusal preserves
   old named output and correct part/cleanup state; foreign part survives.
3. Execute real short/read/write/checked-close/rename/cleanup faults and existing
   work/path/entry boundary controls that changed IO ownership could affect.
   Distinguish injected libc errors, genuine kernel errors and unexecuted hosts.
   Old std-order mutation or retained exact old producer must expose retry again;
   new positive control must refuse before a second native call.
4. Preserve Static65536/Build131072/Domain26/Buildwork7391501569. Audit and count
   new path/callee allocation/stack/readonly/work contributions and all simultaneous
   owners. Do not hide nix1024 CString fallback or extra stack buffers under an
   unchanged label. If exact recipe bookkeeping needs a delta, report smallest
   source-derived proposal before any cap change; retain source/measurement
   distinctions and pending Windows no-op interpretation. Correct concrete same-
   obligation gaps within scope rather than claiming partial native acceptance.
5. After announcing whole immutable Registry+lock candidate to Root, run required
   complete Registry Debug/Release, actual build.rs producer suites, all-target
   Clippy-Dwarnings/fmt/rustdoc-Dwarnings and appropriate affected consumer checks.
   Reuse unchanged green consumer/reference/native evidence transparently; broaden
   only for changed generated output/API/dependencies or concrete concerns. Preserve
   every failure and historical expectation, never regenerate expected facts.
6. Freeze whole new source/lock/delivery and explicit evidence after checks,
   independently reverify original authorities/input/reference/history and private
   outside. Supply exact Round2→Round3 delta/hashes, original context/blocker/
   response, actual syscall/binary/log identities, full native census and remaining
   gates for a new independent review. Author cannot accept itself.

Windows execution, full native attribution review, independent runtime acceptance
and Root integration remain separate gates. World Query53, natural worlds,
Survival, whole FEATURES, Java plugins and equivalent-scope performance are not
established by Registry facts or by a successful Unix open correction.
