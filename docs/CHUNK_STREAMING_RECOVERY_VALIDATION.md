# Preview generation and Play backlog recovery (13 October 2026)

The server can reject a normally paced client after it stops servicing the
connection long enough for TCP input to accumulate. The previous App policy
charged that accumulated input as an immediate flood. A real protocol-772 client
sending 40 frames/s reproduced BudgetExceeded after an externally imposed
16-second pause, while the server process stayed alive. The supplied user trace
is consistent with this failure class; its missing exception cause does not
prove the original host encountered that exact cause.

## Resulting behavior

Established Play retains at most one already decoded frame when tokens run out.
It waits for real configured-rate replenishment and rotates quota retries with
reads, output, Keep Alive, chunks and operator updates. It stops new socket reads
while that body is held. Shutdown and unchanged absolute liveness deadlines keep
priority; deferred or ignored frames grant no liveness credit. Initial handoff
input remains strict across finite output fences and partial decoder tails.

The episode is bounded by `min(burst / rate, 2 seconds)`. Actual half-burst
replenishment can reset it; a held frame's own retry cannot. Continued starvation
ends with typed BudgetExceeded through ordinary save/leave cleanup. Burst and
rate are unchanged. Larger backlogs or actual liveness expiry may still disconnect.
This is a bounded FerrumC policy, not Paper parity. The reviewed pinned Paper
Connection/IntervalledCounter rate policy counts Netty ingress before main-thread
handling; FerrumC's configured token admission remains separate.

Operators now receive the complete connection-error chain at WARN with the peer
address. This diagnostic also covers disconnect causes unrelated to packet quota.

Grass/tree preview cache misses use 32 fixed RAW, 32 BIOME and 32 SOURCE mutex
stripes. A miss rechecks after admission, computes outside cache locks and shares
only completed immutable results. The SOURCE-to-RAW/BIOME lock graph is acyclic;
ordinary refusal releases guards without partial publication and poison refuses
new computation. Content limits and FIFO ordering stay 32/64/32, with no retained
in-flight keys or payloads. Eviction can still require genuine recomputation.
Numerical algorithms, RNG/history, budgets, seed and generator revisions stay unchanged.

## Measured scope

Alternating, uncontended instrumented Release probes on the same accepted source
compare two cold workers. Each case has two observations per variant; these are
CPU measurements, not general TPS, RSS or full-server loading benchmarks.

| Two cold workers | Baseline CPU | Candidate CPU | RAW computations |
| --- | ---: | ---: | ---: |
| Same target | 8.88 s | 4.37 s | 38 -> 19 |
| Adjacent targets | 7.25 s | 5.26 s | 31 -> 23 |

The 22 complete chunk outputs match accepted snapshots byte for byte, including
all 98,304 block states and 1,536 biome names per chunk. Sequential/reverse controls
also match. Three gate tables add 768 native bytes on this Linux target; this is
size accounting, not RSS. Same-target elapsed time is essentially unchanged;
avoided CPU work does not make all cold generation instant.

## Checked source and executable

Clean build base: `4996efc8458da8c47201828927a0a456e19f1054`.
The 1,615-file implementation manifest has canonical SHA256
`1ab71ef05332d1bb57fd5cd6c6ffc2db006cd1e1515078fc9a674aeed901784c`.
Its 396-file World subset has canonical digest
`58242dfd7c646de6b3b3529df6ef72be2a5273cbb0c50313d3ad3bc0fb1cb00b`.
This document is a subsequent evidence summary; implementation bytes stay unchanged.

Standard, uninstrumented Release executable SHA256:
`e573bf0272b0e7703b9268c9c0df5594745ad64489a5af550bbe2c3b864c7d9c`.

- World Release: 445 passed, 7 preexisting ignored instrumentation tests.
- App Debug: 419 passed, 1 preexisting ignored test across 40 suites.
- Workspace formatting, all-target World/App Clippy with warnings denied,
  public World/App rustdoc with warnings denied and standard Release App build: pass.
- Independent source review: CLEAN, including finite-window small-burst cases,
  initial fenced 601-frame refusal, deadline priority and round-robin quota work.
- Final executable socket positive: 40 frames/s, 16-second private server pause,
  872 frames sent, Keep Alive echoed, post-resume sentinel processed, no budget
  rejection or unexpected disconnect.
- Final executable negative: 5,000-frame flood, typed BudgetExceeded after an
  observed 2,014 ms, server still alive. Both private servers shut down cleanly.

The unchanged independent protocol-772 tree harness passed on this exact binary:
169 requested columns initially and after restart, two passable body cells at spawn, refused
body-cell placements, a generated crown across a chunk boundary, one actual oak
log removed and one creative block placed. All 169 initial, edited and restarted
block/biome snapshot summaries match the earlier accepted tree harness baseline.
The initial view contains 1,147 short-grass cells, 3,418 oak logs, 35,239 oak leaves,
1,087 birch logs and 9,387 birch leaves. Saved edits remain after restart.
Full-view arrival took 122.942 / 122.880 seconds initially / after restart on this
host. These are individual observations; loading remains slow. Exploration was
explicitly disabled in this run and has no new acceptance claim.

The launcher warns when an explicit preset meets an existing config. The folder
name does not select a generator. A fresh tree world requires
`ferrumc_natural_tree_preview`; the terrain-only preset omits trees and grass.
Existing config and world identities are preserved, with no automatic migration.
The tree preview remains an oak/birch and short-grass subset; other species,
decorators, complete lighting, mobs and complete Survival are still open.

Evidence is retained under `.codex-tmp/chunk-crash-speed-20261013/`: implementation
manifest, binary record, World/App check logs, independent reviews, final socket
acceptance, paired CPU probes, launcher smoke and client tree acceptance.
Private instrumentation never enters the delivered executable. The initial
public App rustdoc attempt exposed three preexisting private-item links, corrected
to plain references before the final green public-doc check; failed attempts remain.
All unrelated WIP and existing personal world/configuration bytes are preserved.
The personal server remains stopped.
