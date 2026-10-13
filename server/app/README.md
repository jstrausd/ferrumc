# ferrumc-app

Application entry point. Wires config, services, plugin registry, shutdown.

## Durable generation identity

New worlds use storage format4. `[world].seed` accepts an optional signed64
integer and `[world].preset` accepts `"ferrumc_flat"` or
`"ferrumc_natural_preview"`, `"ferrumc_natural_grass_preview"` or
`"ferrumc_natural_tree_preview"`. A fresh omission explicitly
selects seed0 and FerrumC-flat-v1; this fixed layered terrain does not provide
natural Overworld, Nether or End generation. Once stored, the seed is reused
on restart when the setting is omitted. An explicitly different seed refuses
startup before import, spawn, player loading or accepting connections.
The client receives the original Minecraft biome-seed obfuscation of that
stored seed.

Anvil import supplies stored complete chunks before spawn. Chunks absent from
the import use the explicitly declared FerrumC-flat-v1 fallback; importing does
not recover Vanilla's seed or generator settings. Missing or malformed configured
imports fail startup. This importer is not a completed Vanilla world migration.

## Natural terrain preview streaming

The natural preset includes terrain density, biome selection and surface material
rules. Revision1 omits placed features; revision2 adds an isolated short-grass
subset. The explicit tree preview is algorithm2 revision3, adding a bounded
oak/birch subset. It requires a new world: existing revisions cannot be relabelled.
Stored revision3 and seed survive omitted declarations on restart. Other placed
features, placed ores and structures remain incomplete; density ore veins are
included. These preview profiles do not provide complete Survival. Lighting currently supplies vertical
skylight and the motion-blocking heightmap uses highest non-air material. Natural
spawn chooses a dry support column with two clear blocks in the resident spawn
square. Revision3 scans at most384 cells per column through air, short grass,
oak/birch logs and leaves to real terrain, with two passable body cells. Leaves
and trunks cannot become spawn platforms, and other obstructions stop descent.
Earlier revisions retain their top-surface policy. This closed terrain rule is
narrower than original `PlayerRespawnLogic` collision/heightmap semantics. The
revision3 spawn is centered in its checked column so the player body cannot
overlap an unchecked neighboring trunk.
Natural worlds reject Anvil import, including when the preset is restored
from metadata.

Streaming prepares raw terrain outside the simulation task with at most two
blocking producers. Every viewer has up to 64 priorities, scheduled fairly.
Completed immutable baselines occupy an LRU of at most 64 chunks and 8 MiB of
conservatively charged storage (512 KiB per result); generation scratch and
resident chunks are additional. This RAM cache is not on-disk pregeneration.
Normal admission rereads current metadata, full chunks and gameplay overlays, so
cached terrain cannot replay stale edits. Departure and failed joins cancel
pending priorities; shutdown waits already admitted producers.

The client can request a smaller render radius than the configured server cap.
Each pump admits at most 16 chunks or delivery retries and retires at most 16.
One extra retained ring reduces boundary churn; a two-ring speculative margin
prepares likely next columns. Dropped chunk or block-entity packets retry a fresh
resident snapshot while retaining exactly one owned ticket. The initial view
still fills progressively as terrain becomes ready; these bounds do not promise
Vanilla/Paper throughput or complete loader parity.

## Explicit offline format3 migration

Stop the server and close all handles before using the offline command. Normal
startup refuses legacy format3; it never migrates or guesses a historical seed.
The operator explicitly assigns Minecraft1.21.8 / FerrumC-flat-v1 / Overworld /
seed0 to every listed numeric world:dimension scope:

```sh
ferrumc migrate-generation --source world/world.redb --scope 0:0 \
  --declare-legacy-flat-v1-seed-zero --prepare-only
ferrumc migrate-generation --source world/world.redb --scope 0:0 \
  --declare-legacy-flat-v1-seed-zero --backup world/world-format3-backup.redb
```

The backup must be a new explicit file. Migration validates the complete source,
creates and verifies the closed backup, reports its file SHA256 and logical
snapshot SHA256, then atomically commits all descriptors and format4. Keep both
reported hashes. Reuse an existing matching backup only with the recorded
`--backup-sha256 <64 hexadecimal digits>`; existing paths are never overwritten.
Repeat the same command after a lost result: a complete compatible format4 is
fully validated and reported as already compatible. Active handles, malformed
records, incomplete declarations and ambiguous unscoped records are refused.
These offline operations run before normal config bootstrap and bind no sockets.
Backup/rollback limitations follow the storage migration contract; no automatic
repair or rollback overwrites the source.

## Persistence admission

One opaque App target shares the exact verified store allocation among startup,
driver and writer. Ticks reserve bounded queue capacity before synchronous
verified capture; a full queue retains dirty state and journal entries. The
writer revalidates its expected descriptor before writes and retries, including
empty durability barriers. Unload keeps tickets until all previous commits are
acknowledged; shutdown awaits the final worker drain and surfaces failures.

## Invariants

See `INVARIANTS.md` in this directory.
