# GEN-03.1a Round2 — checked close capability and complete Registry implementation

Root implementation continuation, 12 October 2026. Read the complete original
GEN-03.1a task, full original GEN-03.1 contracts/findings/responses/reviews, and
the complete blocked delivery `GEN-03_1a_REGISTRY_STATE_FACTS_RUNTIME_DELIVERY.md`.
All original producer/parser/join/meter/native/publication/tests obligations
remain binding. Only the narrow dependency/write-scope exception below changes.
No World/Heightmap/Orequery/runtime acceptance is granted.

## Capability blocker and explicit Root resolution

Original Rootbrief SHA256 `c01af7e49fd0897b205b7d5aa37e11f6034ab412ec080d442d24494492fbd490`.
G31A-CLOSE-1 blocked delivery SHA256
`2a818fa56a4229b9cde2c3d7aa72ae0bd78db6bfa77895440ca10eb1466c7933`;
blocker evidence manifest SHA256
`c2e80274765ecfa8bc3569e425388f2a183d830a314e784f2356db4da791a36b`.
Root independently verified all1552 regular files/1884 entries, baseline and
unchanged Registry; intake
`.codex-tmp/orchestration/gen03_1a-registry-blocker-intake.json`.
This historical blocked delivery is not a runtime candidate; preserve it fully.

Root authorizes only these exact target-specific **build-only** dependencies
inside Registry, with necessary generated `server/Cargo.lock` additions:

- Unix: `nix = { version = "=0.31.3", default-features = false }` under target
  Unix build-dependencies. Safe `nix::unistd::close(File)` consumes IntoRawFd
  ownership, calls libc::close once and reports the actual errno. Every error,
  including EINTR, refuses publication. Never retry or Drop-close the consumed
  descriptor. Audit full selected dependency/callee implementation and pins.
- Windows: `io-close = { version = "=0.3.7", default-features = false }` under
  target Windows build-dependencies, exclusively its safe `Close for File`
  CloseHandle implementation. Account its flush/no-op bookkeeping and actual
  returned error. No Unix io-close path: that path turns EINTR into success and
  is explicitly not authorized. Unsupported hosts must have an explicit typed
  policy rather than unchecked publication or silent fallback.

No runtime dependency or local unsafe/lint exception. Put justification comments
in Cargo.toml. Lockfile resolution may add required transitive packages/features
only; never update unrelated versions/checksums. Root source decision/pins are
`.codex-tmp/orchestration/registry-checked-close-dependency-decision.json`, SHA256
`aef8edb79c0ca53b1dfb2999d26fb101a87201b71d1b5d05190ae69b348a7cdd`.
Downloaded crate sources are in the normal Cargo cache; preserve their complete
origin/version/checksum evidence. This decision authorizes implementation, not
unexecuted Windows/native/IO acceptance. Do not make a sync/drop substitution.

## Scope, baseline and resumed work

Write only `server/crates/ferrumc-registry/**`, necessary `server/Cargo.lock`
dependency resolution, new
`docs/agent-tasks/GEN-03_1a_REGISTRY_STATE_FACTS_RUNTIME_ROUND2_DELIVERY.md` and
`.codex-tmp/gen03_1a-registry-runtime-r2/**`. No Git or other agents. No old
blocked delivery/evidence edit, other crate, shared build, World/App/Flat change
or additional dependency. Root is the only integrator.

Use the same genuine65c7 archive1505 files/47 Registry/324 accepted World, base
freeze7917226aff23edc427743b73ee8451ae11b3db87faeb11fb2c26178ba1afb2ec.
Copy to fresh Round2 private scope; overlay complete Registry and exact amended
lockfile only. Freeze/hash the lock delta before `--locked` checks. Record whole
immutable outside/baseline membership and bytes; shared concurrent Carver source
changes are external drift, never a build source or globally frozen claim.

First compile and exercise the actual checked-close adapter using owned real
files in Debug/Release. Capture exact error and ownership/no-retry/no-doubleclose
behavior. Actual syscall-boundary failure injection may be test-only native
instrumentation in separate processes, like prior allocator harnesses, with
explicit positive controls; never call a fake IO trait result a kernel failure.
Separate real kernel errors, injected libc/CloseHandle errors and unsupported/
unexecuted platforms. Keep old output unchanged across refusal and cleanup.

Then complete the original Registry implementation: bounded hash-before-load,
full1105-name/27946-state property/domain/radix identity join, exact flag/fluid/
default/membership closure, static facts/CheckedStateId, bounded metered streaming
and atomically named output, full malformed/all-ID/native/work/publication tests.
Maintain accepted design limits Static65536/Build131072/Domain26/Buildwork7391501569.
Existing loader/emitter and borrowed normalized metadata remain separately
measured phases. If adapter bookkeeping needs an explicit additional debit,
include its exact source-derived census and report the smallest delta; do not
silently hide work or change a frozen formula. No World reservation is accepted.

Full Registry Debug/Release/strict checks and appropriate generated-output
consumer checks follow original assignment, private locked/offline/jobs2 target.
Notify Root at whole final candidate before checks; freeze complete new Registry
source, exact lockfile, new delivery and evidence only after checks pass and
immutable original authorities reverify. Preserve all failed/superseded attempts.
Deliver concrete source/API/check/native/IO evidence and all remaining gates;
no runtime self-acceptance, natural-world/Survival/whole FEATURES/Java-plugin or
performance claim. Do not stop for routine permission within this exact scope.
