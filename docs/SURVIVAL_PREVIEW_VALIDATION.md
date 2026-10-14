# Local Survival preview validation — 14 October 2026

The explicit `ferrumc_natural_survival_preview` preset now connects revision-4
terrain, grass, oak/birch trees and placed ores to the native Survival runtime.
Fresh players have empty inventories. Timed mining, owned ground drops/pickup,
basic crafting, chest transfers, furnaces, health/hunger/respawn, water/buckets,
tracked melee and the adult sheep/cow/pig/chicken/zombie subset are active.
This is a bounded early-game preview. Complete Minecraft/Paper behavior, all
item actions, XP/enchanting, lava, structures, dimensions and Redstone remain open.

Use a new directory for this preset; existing configs and generation identities
are preserved. [Launch instructions](LOCAL_NATURAL_WORLD.md) include local and
other-host commands. Stop with Ctrl-C and wait for the process to exit before
reopening a save. Player, entity and block persistence are separate writes;
graceful-restart tests do not establish crash-atomic pickup or death.

## Final artifact

The tested release binary is SHA256
`fba70091258a4dd652485159416eafa4e38e72e817c608031f022060cd91a6f3`.
Its complete 1341-member server source/input snapshot is SHA256
`6db9d6e51210ec6e0f8fbc7d0d2dd0f7e1fe9049358d5ec4110e85ccaebb3d06`.
The base is `81b6bd88647fd8a4fc1cbe7fa4bf479cd288e9d9`. Unrelated Registry,
ore-placement and other work in the main working directory was excluded.

Local retained artifacts live under `.codex-tmp/survival-night-20261013/`.
`final-survival-binary2/source/`, `snapshot.json` and `source-before-build.json`
are read-only. Binary1 remains explicitly superseded before runtime acceptance;
the original two-break mining-wear failure and unsuccessful probe attempts are
retained with their dispositions.

## Final binary client checks

These tests use independent protocol-772 clients and isolated private worlds.
They decode real packets and exact item representations; they do not verify
graphical rendering or a running Paper server.

| Check | Actual observed result | Retained evidence |
| --- | --- | --- |
| Fresh natural join/view | Empty Survival inventory; standing view fills all169 columns without Forget; eight keepalives answered; 81 whole block/biome columns equal the earlier coherent terrain evidence. | `world/final-join-view/FINAL-ASSESSMENT.json`, SHA256 `484483755cafde5bad3f840e38c24ee0fd1ad07ac4e38012d6c2a14480c5360e` |
| View retention | Actual centers `(0,0)→(0,1)→(0,2)`; old radius7 ring retained; 13 Forget packets retire only radius8 cells; 195 unique columns and no duplicate sends. | Same join/view assessment and raw packets/chunks. |
| Mining burst | A commits with DAMAGE1; overlapping B receives authoritative correction and ACKs; retry after A's receipt commits B with cumulative DAMAGE2. | `acceptance-extra/miningwear-guard-1/` |
| Mining followed by attack | Real START+FINISH→ATTACK burst mines once and refuses the overlapping attack; victim remains20, tool DAMAGE1. Retry after the positive mining receipt reduces health to17.5 and produces cumulative DAMAGE3. | `inventory/mining-attack-finalbinary2-evidence/manifest.json`, SHA256 `c9bda43eb7c46846e2172d0af1623fcd342ac954e27edfcd93d0b3b74ac3c3e6` |
| Q / Ctrl-Q | One/whole selected-mainhand drops preserve full raw components, actual Collect1/7 and pickup delay. Final46 raw slots and settled cursor match after restart; no duplicated ground item. | `acceptance-extra/FINAL-BINARY2-ASSESSMENT.md`, SHA256 `e15999468bdb795d55f684ab6b43ee3fb6ba430f0378b83d21c7eafd0e1a2aed` |

The view test uses disclosed short protocol relocations to verified dry supported
destinations. It does not prove intervening walking, gravity, climbing or
anti-cheat. Its full initial169-column fill took51.653seconds in a concurrent
run; this is an observation, not a quiet throughput/TPS/RSS comparison.
The Q test settles the glass cursor with a real inventory click before capturing
its final image: normal disconnect cleanup returns carried stacks to storage.

## Earlier coherent gameplay evidence

The earned natural-world chain is separately pinned to predecessor binary
`cefbf4a0d251e48b8ac7fcc85dce07697e457263844f39ea956ad1961e10115d`.
It starts empty in Survival, mines six actual logs, crafts planks/sticks/table
and a wooden pickaxe, mines eleven cobblestone, crafts a stone pickaxe/furnace/
chest, mines natural iron, and smelts an ingot with earned planks. Both chest
transfer directions, 36 concrete pickups and 30 exact tool-wear checks pass.

Same-world restart preserves all46 player slots/cursor,27 chest slots,3 furnace
slots and health/food/saturation. All81 biome hashes match. One terrain-cell
difference is the independently observed normal lit→unlit furnace transition
after resumed fuel expiry; other cells remain exact. The original failed raw
oracle is retained. The final read-only assessment is
`world/early-survival-acceptance/FINAL-ASSESSMENT.json`, SHA256
`5ca2a4d32f1878c0f3a8f577b35b0066dfa16a1c09c03f89cc48d9a88d8be0b0`.
All156 final World production-source members equal that predecessor, and the
final-binary view test independently verifies its81 pristine column hashes.

Earlier coherent private checks also cover computed torch/skylight changes,
water flow/source-only bucket pickup, furnace progression/resume, melee wear,
actual mob movement/drops/save, hunger/food and death/respawn. Their exact binary
and scenario boundaries remain in `acceptance-extra/extra-evidence.json`.
The specific pending-food/deferred-effect timing race remains unproven as a live
client scenario; accepted and refused owner-result regression tests cover the
retained positive mining witness. Earlier checks are not relabelled as final
binary executions, and the earned chain was not repeated unnecessarily.

## Checks and input ordering

The final App run passes571 tests with0 failures and1 preexisting ignored test.
The selected unchanged supporting-crate suites pass800 tests, Sim/Session/
Placement841, and World Release476 with8 preexisting ignored tests. Final App
all-target Clippy with `-D warnings`, workspace fmt, strict App rustdoc, generated
protocol freshness and release build pass. Relevant broader strict checks were
already green before the final App-only correction.

Connection actions that can replace, remove or wear a mining tool temporarily
refuse while its terminal receipt is unsettled. This includes attack, inventory
exchange, held drop/swap, item use and commands. Inventory/block predictions are
healed; movement, liveness, finish/abort and selection remain available. Creative
mining has no tool-wear witness and keeps instant pipelined input behavior. The
unchanged two-client Creative block edit/refusal test passes. This conservative
receipt policy is FerrumC behavior, not original Paper tick equivalence.

Private test servers stopped cleanly. The personal server was not started and
personal worlds/configs were not edited.
