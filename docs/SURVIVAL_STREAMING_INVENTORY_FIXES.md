# Streaming, mining pickup and crafting corrections — 2026-10-14

Base: `86a72f7d6e60be79d625114a0b483955cefdfc83`. Changes are confined to
`ferrumc-app`; generator revision5, dependencies, durable schemas and existing
world identities remain unchanged.

## Corrected behavior

- Background terrain preparation previously stopped the simulation driver on
  any block entity. Revision5 cabins legitimately contain loot chests. Clean
  generated chest/furnace/sign baselines now require matching block states and
  a valid complete native payload. Dirty/edited baselines, lighting, invalid
  entities and wrong column positions still refuse. The unchanged cache limits
  (64 entries, 8 MiB total, 512 KiB per entry) charge the complete decoded entity,
  component and recursive NBT clone; normalized clones preserve all contents.
  A valid oversized result is memoized without terminating the driver.
- Ground pickups now merge matching selected/offhand stacks, then other matching
  inventory stacks, before choosing empty hotbar slots0..8 and finally the main
  grid. Whole save-budget refusal returns the original offered stack.
- A pickup no longer cancels mining when the exact selected tool stack is
  unchanged. A real change still cancels work and preserves the unsettled wear
  witness until the owner's terminal receipt. Receipt serialization and tool
  damage rules are unchanged.
- Ordinary left/right QUICK_CRAFT now works in inventory2x2 and workbench3x3
  screens. Start/add phases do not mutate items; finish admits the complete
  player/cursor/table component budget before one staged publication and recipe
  derivation. Duplicate, invalid, stale-screen or changed-cursor phases do not
  create items. Old StateIds and client slot predictions are not authority.
  Equipment is published only when actually changed, including refused drags.

## References and limits

Pinned Paper1.21.8 build60, source commit
`29c8822d90899c89d2689338e81a98f690bcba12` (see `PAPER_REFERENCE.md`).
Complete original `Inventory.getSlotWithRemainingSpace`, `getFreeSlot`,
`addResource` and `add`, their Inventory source patch, and the previously read
complete `AbstractContainerMenu.doClick` QUICK_CRAFT / `handleContainerClick`
paths guide ordering and phase behavior. Original Inventory SHA256:
`e6573a8c2c222b574d675d72ff190645996ac56172bbcadbfed3fb9ee07a95ba`;
Inventory patch: `d342fd905a4aae654e4eed1c048938458277125c9e52ff64577982b0d09b0f2b`.
The complete patched `ServerPlayerGameMode` tick/progress/action paths were read
for mining context (SHA256 `fa54987063728f7d79c6a522509f00dce63de2836bea551645343d791b5becae`).
This change fixes the App's spurious tool-change input; it does not replace
mining hardness/timing or implement original Paper lag compensation.

These are FerrumC regressions, not graphical client or Paper differential
acceptance. Creative middle-fill, furnace drag, Java inventory events and
player/world crash-atomic exchanges are separate unimplemented behavior.
Existing conservative mining receipt serialization remains in force.

## Passed final validation

Final Release binary SHA256:
`f46f2f2023e627df159008a0ec200fd15ff4cfb5edf6fe174543904922296a5f`.
Final server source:1343 files, canonical SHA256
`ff388bcbfd03cdf0e22fe4f903953b9c388a87bc6491f9d14f2156a44970d96a`.

- Complete App library:496 passed,0 failed/ignored.
- Real protocol inventory suite:3 passed, including rapid2x2/3x3 operations.
- Release preparation suite:14 passed, including genuine seed0 cabin
  `(-150,97,-149)` through background producer/cache/ordinary owner admission,
  edited loot and redb close/reopen with a fresh producer and owner.
- Formatting, App all-target Clippy with warnings denied and rustdoc with
  warnings denied passed; locked/offline builds with2 jobs.
- Final-binary protocol772 mining: iron-pickaxe stone break, start iron-block
  mining before the stone pickup, actual pickup into hotbar1 while cracks
  continue, second committed break, actual drop into hotbar2 and DAMAGE2.
  Both air cells, drops and damage survive graceful restart. Item spawn and
  block removal were observed together for both breaks; first pickup arrived
  about414 ms after the observed commit. This is a bounded functional probe,
  not a general performance benchmark or removal of the normal pickup delay.
- Final-binary rapid same-StateId2x2/3x3 left/right drags, actual recipe
  consumption and exact player/cursor restart passed.
- Final-binary natural cabin in a neighboring streamed column, independent
  fixed loot, real loot transfer and edited chest/player restart passed.

Evidence: `.codex-tmp/survival-fixes-20261014/`, including source manifest,
final logs and three protocol result/packet/provenance directories. Earlier
failed checks and the superseded first candidate are retained separately.
The old published binary reproduced the wrong empty-slot ordering; the
combined suite also caught a redundant equipment publication on a refused drag,
which was fixed without changing the old rejection assertions.

Personal servers, configs and saved worlds were not used. Foreign Registry/
World WIP was excluded from builds and preserved during integration. The
no-argument launcher and stored generator identity retain their previous behavior.
