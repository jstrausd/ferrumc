# Natural local terrain preview — 2026-10-13

The operator requested a locally testable natural world before noon Vienna
time, with delegated implementation. World, Sim and App agents implemented
separate scopes; Root supplied config selection, the storage unknown-kind test
adaptation, isolated integration, launcher, deployment and independent checks.
The local release server was ready at 10:41 CEST on localhost:25566.

## Delivered behavior

- Explicit `ferrumc_natural_preview`, algorithm 2/revision 1, Minecraft Java
  1.21.8 / protocol 772; seed 0 in a separate local world.
- Accepted noise materials, genuine 1536-quart biomes, surface rules and cold
  carvers compose into complete chunks before publication. Producer failures
  propagate without partial residency or flat substitution.
- Cancellation-safe, bounded offthread admission; one nonresident column per
  stream command, with later client requests filling deferred positions.
- Dry surface spawn; complete configured biome mapping; surface heightmaps
  and approximate vertical skylight. Exact stored seed/preset identity gates
  restart. Natural Anvil import refuses before imported writes.
- Separate local release executable/config/save; existing server worlds and
  defaults remain intact. [Run instructions](../LOCAL_NATURAL_WORLD.md).

## Verification

Integration used genuine accepted main `ff2d0337de1318a9f4fd861cb32cfaea7c8dd490`,
an isolated 1562-file archive and a 29-file preview overlay. All 1535 files
outside that overlay remain byte-identical. Unaccepted Registry, ore-placement
and shared lockfile work was excluded. The final README wording clarification
changes documentation only; the final release executable remains byte-identical.

Root checks, locked/offline with two build jobs:

| Check | Result |
| --- | --- |
| Workspace all-target tests | 3128 passed, 0 failed, 8 existing ignored, 169 suites |
| Warnings-as-errors workspace doc tests | 89 passed, 0 failed, 35 suites |
| Workspace all-target Clippy `-D warnings` | Passed |
| Workspace format check | Passed |
| Generated protocol consistency | Passed |
| Public-claim guard, normal and Python `-O` | Passed, including 125 self-test cases |
| Existing independent flat smoke | Passed, including all 28 pattern blocks after restart |
| Natural independent client, Release and Debug | Passed: creative edit/ACK, ±X exploration, complete terrain/biomes, restart |
| Deployed local server | Status 772, login, dry spawn (8,63,8), nine spawn chunks |

Author checks separately passed World 424 tests, Sim 415 and App 293, with
Clippy/format. Seven original FINAL groups cover origin/seams/negative chunks
and signed seed extremes; generated blocks match the committed original
computational evidence. These fixtures do not establish whole live Paper parity.
Read-only reciprocal reviews found no blocking integration defect and recorded
the limits below. The CI workflow now runs the independent natural client gate.

Deployed binary SHA256:
`5fce21be777ca75043106e7cd273d4ead2478c74237c62c15ede871957b82be6`.
The client placed gold at (8,63,10), confirmed state 2137 plus ACK, then proved
the saved edited chunk hash and all quart biomes after graceful process restart.
Chunks (1,0) and (-1,0) also regenerate identically across restart.

Local evidence is retained under `.codex-tmp/natural-local-20261013/`:
`final-verification.json`, `combined-integrity.json`, `overlay.json`,
`test-summary.json`, both independent review reports, full check logs and
`client-final-doc/acceptance.json` / `client-debug/acceptance.json`. Earlier
attempts remain available. The separate `local-natural/delivery.json` records
the deployed process/config and executable digest.

## Remaining scope

Placed-feature ores, trees, vegetation and structures remain unpopulated;
material-stage noise ore veins are present. Nether, End, mobs and complete
Survival remain future work. Lighting omits transparency, lateral propagation
and emitted block light. Both heightmaps inherit highest-non-air semantics,
so powder-snow motion-blocking fidelity is incomplete. The driver waits for
each generation result and can pause ticks during exploration; no 20 TPS,
performance improvement or all-seed parity is claimed. Full natural snapshot
biome persistence is unsupported; the guarded App path uses generated baselines
and edit overlays.
