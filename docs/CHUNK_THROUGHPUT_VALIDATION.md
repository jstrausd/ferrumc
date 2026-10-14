# Natural chunk throughput validation — 13 October 2026

Natural chunk generation and loading are faster while preserving existing world
identities, complete tested block states, biomes and saved edits. The final
Minecraft 1.21.8/protocol 772 executable was built from a clean archive of main
at `8ec4cd94ed473855a2277b3a67d74fd1c7f183ab` plus the reviewed World/App changes.
No generation reset or migration is required.

## Measured result

| Check | Accepted baseline | Final candidate |
| --- | ---: | ---: |
| Cold single tree chunk, standalone generation | 4.177 s | 2.658 s |
| 16 sequential tree chunks, standalone generation | 12.937 s | 8.054 s |
| Full 169-column view, fresh world | 122.942 s | 82.914 s |
| Full 169-column view, saved restart | 122.880 s | 81.716 s |
| Entered columns ready before 12 walking crossings | 4/12 | 12/12 |
| Next columns ready before those crossings | 0/12 | 12/12 |

The scoped generation cases use 36–38% less time. Full-view observations use
32.56%/33.50% less time. These are individual same-host measurements, not general
TPS/RSS guarantees or a comparison with a Paper server. Full initial loading
still takes time; this does not make natural generation instantaneous.

The unchanged independent movement probe starts five seconds after initial
position sync and moves 5.6 blocks/s at 20 Hz without waiting for missing terrain.
Every central and next column was ready one block before all twelve positive-X
crossings, and immediately before each actual crossing packet. The smallest
measured next-column lead was 474 ms. All 35 complete overlapping baseline
corridor snapshots match. A brief author check overlapped baseline startup, so
its timing is not an isolated CPU benchmark; its missing-column observations
remain valid. No builds overlapped the final movement or full-view measurements.

The unchanged `tests/blackbox/natural-world.mjs` tested a fresh seed-0 revision-3
tree world with view distance 6, exploration disabled and a genuine creative
fixture. All 169 initial, edited and restarted full-column snapshots equal the
accepted baseline: 507 complete block-state/biome comparisons. The test removes
an actual generated log, checks the stored edit, keeps the full view across live
time updates and performs a graceful saved restart. Private servers stop cleanly.
This is protocol/creative-preview acceptance, not GUI, physical Survival or full
Paper behavior. Repeated exploration is outside this run's scope.

## Implementation and memory

A seed/settings-bound cache retains at most 64 immutable fresh-noise climate
columns. Each biome traversal still replays original source selection history,
work and refusal behavior. Seven pure distance operations are grouped only when
a public allowance admits the entire group; partial allowances and private
Carver meters keep their original unit sequence. Existing RAW32/BIOME64/SOURCE32
limits and generation revisions remain unchanged.

The climate cache has 4,724,008 bytes of native capacity, excluding allocator
overhead, active leases and constructor scratch. Standalone observed peak RSS
increased from 12,100 to 15,304 KiB cold and 13,280 to 18,124 KiB in the sweep.
The bounded cache trades additional memory for CPU time; it is not an RSS cap.
All 17 standalone complete generated outputs equal the captured baseline.

Visible requests precede speculation across viewers. The pump scans up to 64
candidates while the owner independently caps successful acquisitions at 16
columns; resident retries share that connection allowance. Two blocking jobs,
64 warm baselines and 8 MiB charged storage remain the preparation bounds.
Successful owner admission frees redundant raw copies. Each viewer's bounded
wanted set protects shared demand; replan/cancel removes globally obsolete
entries. Completed producer shape is validated before dropping obsolete or
already-resident output. Stored snapshots/overlays are reread during acquisition;
preparation neither owns tickets nor replaces gameplay state.

The current column, next two forward columns and their lateral margins precede
ordinary side/rear rings. Unknown-heading cardinal priority applies only to the
second stationary ring; other stationary rings keep their previous order.
This preserves outer scheduling locality. An earlier full-ring priority attempt
was rejected after a slow/incomplete full-view run; its source, executable,
checks and interrupted test logs remain preserved. The final measured outcome
does not by itself prove the suspected cache-thrashing mechanism.

Pinned Paper Moonrise player chunk loading and task scheduling implementations
were read. Paper separates load/generate/send stages, reuses/promotes tasks and
skips unchanged plans. FerrumC's movement policy is explicit and does not claim
Paper scheduling parity. Repeated stationary replanning and duplicate section
encoding remain profiling opportunities.

## Checks and evidence

Final complete App Debug: **431 passed, 0 failed, 1 ignored**. The existing
external Java-plugin fixture requires explicit Java/JAR paths and was not run.
World Release: **449 passed, 0 failed, 7 ignored**, reused from the prior whole
run because all 396 final World source files are byte-identical. Workspace fmt,
World/App all-target Clippy with `-D warnings`, public rustdoc with warnings as
errors, and the standard Release App build all passed on the final implementation.
The rebuilt executable is byte-identical to the one used by final runtime probes.

Independent source, movement and full-view evidence reviews found no blocking
issues. All 507 full-view snapshots and 35 overlapping baseline movement snapshots
were independently compared. Source checks and scoped runtime acceptance are
recorded separately; neither proves untested Survival or Java/Paper behavior.

Final implementation manifest: `21ebc5aeca2df0a1bfb43a7e5e8722537caccc12691198b4ca0c98f836e9e707`
(1616 complete clean source files). Final executable SHA256:
`c8873df6edb27af906762f3ac63f24b29bc031e62ade62ae6d4756bc0f2456ba`.

Evidence is retained locally under `.codex-tmp/chunk-throughput-20261013/`:
`candidate.source.json`, `binary.json`, `world-speed/final-candidate-results.json`,
`candidate-movement3/result.json`, `client-trees-view6-2/acceptance.json`,
`acceptance-comparison.json`, final checks and independent review reports.
Superseded candidates are explicitly separated from final acceptance.

## Running the update

```sh
git pull --ff-only
cargo build --manifest-path server/Cargo.toml --release --locked -p ferrumc-app
./scripts/run-natural-preview.sh
```

Keep the existing world/config directory. Trees and short grass require the
revision-3 `ferrumc_natural_tree_preview` preset; the launcher does not replace an
existing preset when an environment setting changes. Full Survival, entities,
other placed features, full lighting and Java-plugin world/inventory APIs remain
separate work.
