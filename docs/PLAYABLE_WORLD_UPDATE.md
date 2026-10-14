# Playable-world update

The normal launcher is `./start.sh`. It creates the
latest natural playable profile in `local-play/` and exports `RUST_LOG=debug`
unless a logging filter is supplied. After updating source, rebuild the standard
release binary once; the launcher selects it automatically.

Old `local-natural/` preview saves remain separate. Their folder names did not
select their generator, and existing identity metadata remains authoritative.
A revision change never silently changes an old baseline beneath saved edits.

## Inventory interaction

Vanilla can predict multiple container clicks before receiving the first update.
Those clicks share an older StateId. The server now executes valid actions in
order against its own current slots and cursor, then sends authoritative content.
Wrong windows, inaccessible containers, spectators and conflicting chest
snapshots remain guarded. Client changed-slot predictions are never used as
item authority.

Ordinary chest left/right drag records at most 63 selected slots, mutates nothing
during start/add, then submits one complete checked chest/player/cursor plan.
The complete player-save envelope and persisted chest representation pass
before publication. Invalid sequences or a conflicting viewer retain items.
Creative middle-drag and Java inventory drag events are outside this change.

## Generation scope

`ferrumc_natural_playable` selects a new revision 5 baseline. The existing
terrain, biome, surface, cave, grass/tree and ore producers remain available
under their old versioned identities. Additional vegetation and surface
structures are native deterministic additions. They are not a claim of complete
Vanilla seed parity, complete structure catalogs or original FEATURES chronology.

The additions include biome-routed spruce, jungle, acacia, dark/pale oak,
mangrove and cherry wood/leaf trees; grass, flowers, ferns, berry bushes,
mushrooms, cactus and water-adjacent sugar cane. Desert wells, swamp-hut
shells and original abandoned cabins with typed loot chests are generated.
See [the exact scope and remaining gaps](PLAYABLE_WORLD_GENERATION.md).

## Verified update (14 October 2026)

The final release executable was tested through an independent 1.21.8 protocol
client in private persistent worlds. Rapid right clicks, left/right drag,
bidirectional shift transfers and complete chest/player inventories survived
restart with all 64 stone and 16 glass conserved. Nine real natural columns
contained grass, logs, leaves and underground ores; every block/biome hash
matched after restart. The seed-0 cabin chest at (-150, 97, -149) contained
bread 1, coal 8, iron nuggets 3, wheat 5 and an oak sapling. Taking its bread
and restarting preserved both the edited chest and the player's item.

484 App library tests, 4 inventory protocol tests, 20 item-plan tests, 2 new
Sim owner tests, 8 new World tests in release, 19 generation identity tests
and 8 configuration tests passed. Strict all-target Clippy for the changed
packages, workspace formatting, rustdoc with warnings denied and launcher
mock checks passed. Prior failed attempts remain in private evidence. The
restart probes wait for an actual periodic owner tick after join metadata;
they do not accept a pre-tick container request as gameplay readiness.

Evidence is under `.codex-tmp/playable-world-20261014/`; the binary SHA-256 is
`37e3c0ba735fd1da74f913a53ee1b425f9fdef750cd52318f2ea2d9a6b4847b6`.
These checks establish the implemented subset, not graphical acceptance,
complete Vanilla generation or Paper differential parity.
