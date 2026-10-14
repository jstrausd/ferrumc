# Local natural Survival world

The normal launcher selects `ferrumc_natural_playable`: versioned natural
terrain, biomes, surfaces, caves, vegetation, trees, ores and the implemented
surface structures. It uses seed 0 for a new local save. The underlying
server default remains `ferrumc_flat` when no world profile is configured.

## Run locally

From the repository root, the normal startup needs no arguments:

```sh
./start.sh
```

Fresh runs create `local-play/config.toml` and save in `local-play/world/`.
They select `ferrumc_natural_playable`, the latest revision with grass, trees,
ores and the implemented structure subset. `RUST_LOG=debug` is the default;
set `RUST_LOG=info` if you want less output. The old `local-natural/` previews
are left intact, so a previously terrain-only config is not accidentally
selected as the normal play world.

Connect a **Minecraft Java 1.21.8** client to **localhost:25566**, or this host's
address from another machine. Minecraft and an enabled web dashboard bind to
`0.0.0.0`; the launcher enforces this for existing configs too. Stop with
Ctrl-C and wait for the process to exit so world/player changes are saved.
A second process cannot open the same database.

After updating a checkout, build once and then use the same short startup:

```sh
git pull --ff-only
cargo build --manifest-path server/Cargo.toml --release --locked -p ferrumc-app
./start.sh
```

The launcher prefers `server/target/release/ferrumc` and falls back to
`local-natural/ferrumc` when no ordinary release build is available. It prints the executable,
configuration, generator and logging filter before starting. Building refreshes
`server/target/release/ferrumc`; no binary environment variable is needed for
that ordinary build. The launcher never builds source automatically.
For a custom Cargo target directory, set `FERRUMC_SERVER_BIN` to its executable.

Existing configurations and world generation identities are always kept.
Renaming a terrain-only directory to `tree-preview` does not change its generator.
If an explicitly selected legacy config uses an older preset, the launcher
shows that preset and explains how to select a fresh directory. It does not
rewrite old terrain, player edits or configs. Natural spawn Y is selected on
a dry surface in the configured spawn ring; configured X/Z guide the search.
An all-water ring fails startup with an actionable error.

Optional overrides are available when needed:

```sh
FERRUMC_SERVER_BIN=/absolute/path/to/ferrumc \
FERRUMC_NATURAL_DIR=/absolute/path/to/fresh-world \
FERRUMC_NATURAL_PORT=25568 ./start.sh
```

Use `./start.sh --help` for all settings. Do not append
`--bind`: both listeners already bind to `0.0.0.0`. Fresh configs leave the
web dashboard disabled; enable `dashboard_enabled` while the server is stopped
if you want it. Its configured default port is 9090.

## Survival terrain preview

The no-arguments launcher creates the latest playable generator. To test it in
an additional fresh world, only select a separate directory:

```sh
FERRUMC_NATURAL_DIR="$PWD/local-play-second" ./start.sh
```

Fresh players start in Survival with an empty inventory. The native preview
implements timed mining, owned drops/pickup, basic crafting and chest transfers,
furnaces, hunger/health/respawn, tracked melee wear, water flow/buckets and adult
sheep/cow/pig/chicken/zombie gameplay. Its separate `entity-runtime.redb`
sidecar retains canonical item/mob identities and mob health on graceful stop.
Use Ctrl-C and wait for the process to exit before reopening the same save.
Entity, player and block writes are separate; this is not crash-atomic pickup
or death persistence. Complete mob AI, all item actions, lava physics, the full structure catalog,
Redstone, complete Vanilla/Paper behavior and the full Java-plugin API remain unfinished.

The same stored preset must be retained on restart. Setting the environment
variable for an existing config does not change that config or its world identity.

## Configuration and persistence

Set all top-level settings before the `[world]` table:

```toml
world_dir = "fresh-natural-world"
spawn_chunk_radius = 1
view_distance = 6
chunk_stream_interval_ms = 50

[world]
seed = 0
preset = "ferrumc_natural_playable"
```

A stored generation identity remains authoritative. Omitting seed/preset on
restart reuses it; explicitly changing either refuses startup. Select a
separate fresh `world_dir` to test a different seed or generator. Legacy
format-3 migration remains an explicit offline operation. Anvil import is
rejected with natural preview because its full imported snapshots currently
do not preserve the required biome payload.

Untouched terrain regenerates from the same versioned seed. Player edits
are durable overlays on that baseline. Generation runs on a blocking worker
with bounded admission; failed generation installs no partial chunk or flat
replacement. Two bounded background jobs prepare new columns without awaiting generation
inside the tick loop. Delivery retries do not acquire duplicate chunk tickets.
The client keeps the requested view plus one retention ring; two additional
rings are prepared ahead of movement. Prepared raw baselines have a 64-column /
8 MiB charged cache. Resident gameplay chunks and worker scratch are additional.
Far chunks may unload from memory after edits are durably saved; retaining the
entire explored world in RAM would defeat the memory target.

To test the optional grass revision, create a separate fresh directory and set
`preset = "ferrumc_natural_grass_preview"` under `[world]`, or use:

```sh
FERRUMC_NATURAL_DIR="$PWD/local-natural/grass-preview" \
FERRUMC_NATURAL_PRESET=ferrumc_natural_grass_preview \
./start.sh
```

The preset variable selects newly created configuration only. Existing revision-1
saves cannot be silently upgraded: their generation identity is deliberately
checked before startup. Short grass is passable for spawn and replaceable by
other blocks during placement.

To test oak and birch trees together with grass, create a fresh revision-3 world.
The directory name does not select a generator: `ferrumc_natural_preview`
generates terrain without trees or grass, even in a directory named `tree-preview`.
Use a new directory if that name already contains a terrain-only world:

```sh
FERRUMC_NATURAL_DIR="$PWD/local-natural/tree-preview-v3" \
FERRUMC_NATURAL_PRESET=ferrumc_natural_tree_preview \
./start.sh
```

This selects a separately versioned tree subset; existing revision-1/2 saves
keep their identities. The launcher preset applies only when creating a new
configuration. Spawn chooses dry ground below the canopy with two passable
body cells, rather than placing the player on leaves or inside a trunk.
Trees can naturally be absent in biomes that do not admit the implemented
routes. This preset is not complete Minecraft tree or Survival generation.

A normal client can accumulate packets during a server pause. Established Play
uses a short, bounded catch-up window to drain those frames at the configured
packet rate. It retains one frame, keeps absolute liveness deadlines, and still
disconnects a sustained flood. This cannot mask an arbitrarily long stall.

If a client reports `Connection reset`, keep the server output around the
disconnect and check whether the server process is still running. The client
message alone does not identify a process crash. For a reproducible log:

```sh
set -o pipefail
RUST_BACKTRACE=1 ./start.sh 2>&1 | tee server.log
```

This keeps existing saves and configs. A preset environment variable cannot
upgrade a previously generated world; the launcher warns when reusing its config.

Player placement checks final collision shapes against live nonspectator
players before any mutation, including refined slabs and both door/bed cells.
A refusal resynchronizes the prediction and acknowledges the action. Original
offset bamboo/dripstone/moving-piston shapes remain unsupported and fail closed.

## Preview limits

- The Survival preset adds a bounded ordinary ore subset; complete feature
  chronology and the full structure catalog remain unfinished. The playable
  profile adds desert wells, swamp huts and native abandoned cabins with loot.
  Older revisions keep their
  original ore population. Material-stage ore veins remain present. An optional `ferrumc_natural_grass_preview`
  preset adds original isolated plains/forest short-grass features (revision 2).
  This is a subset, not complete vegetation or full feature chronology.
- The optional `ferrumc_natural_tree_preview` adds undecorated configured oak
  and birch shapes. It omits fancy oak, fallen trees, bee nests, leaf litter and
  other tree species. Its isolated-source placement policy is versioned and
  does not reproduce the complete original FEATURES stage.
- Overworld only. The adult sheep/cow/pig/chicken/zombie subset has simplified
  movement and melee; complete mob goals, Nether, End and the dragon remain open.
- Resident computed skylight and emitted block light propagate after edits.
  Heightmaps use the current highest-non-air model; complete original lighting
  scheduling and all translucent block behaviors are not accepted.
- Supported block-entity families receive default render descriptors after
  joining. Custom banner patterns, skull profiles, pot decorations and shulker
  contents require separate behavior; default descriptors do not implement it.
- Initial spawn loading and exploration still take generation time. Background
  admission and caching reduce tick blocking and repeated work; no unmeasured
  global TPS/RSS improvement or complete Paper loader parity is claimed.
- The original cold context excludes structures/blending and complete chunk
  status scheduling. Source references and differential library fixtures do
  not establish complete live Paper or all-seed world parity.

## Independent client check

With the existing `tests/blackbox` Node dependencies installed:

```sh
MC_SERVER_BIN=/absolute/path/to/ferrumc \
MC_RUN_DIR=/absolute/path/to/new-test-directory \
MC_NATURAL_VIEW=6 \
node tests/blackbox/natural-world.mjs
```

The test owns its process on port 25567 and records its acceptance JSON and
server logs under `MC_RUN_DIR`. It uses independently maintained protocol and
chunk decoders; its result is scoped to the tested terrain, exploration and
restart behavior, not complete gameplay parity.

For the tree subset, add `MC_NATURAL_PRESET=ferrumc_natural_tree_preview` and
use a fresh test directory. The tree check also verifies a crown spanning the
negative-Z chunk boundary, removes an actual generated log in creative mode,
and compares all requested block states and quart biomes after a saved restart.

## Current tree loading measurement

The 13 October throughput optimization keeps the same revision-3 tree world and
32/64/32 raw/biome/source cache limits. Existing `tree-preview` saves can be
restarted with the updated executable and the same launch command above.

In the standard seed-0 protocol test, all 169 view-6 columns loaded in
**82.914 seconds** initially and **81.716 seconds** after restart, versus
122.942/122.880 seconds in the previous accepted run (32.56%/33.50% less time).
Complete generated block/biome snapshots and saved edits stayed identical.
At 5.6 blocks/s, all 12 entered chunks and their next forward chunks were ready
before each tested crossing. Standalone cold/sequential generation used 36–38%
less time, with a bounded additional climate cache of about 4.5 MiB native capacity.

These are individual same-host observations, not general TPS/RSS or Paper-server
benchmarks. Initial loading still takes time; other species, decorators, leaf
decay and full Survival remain open.

See [the exact validation and evidence](CHUNK_THROUGHPUT_VALIDATION.md).

## Joining above loaded ground

Before the initial position sync, the server now sends and flushes the actual
fresh or restored destination column. Cold generation retries observe one
absolute join deadline and shutdown; a missing column refuses the join rather
than releasing the player over unloaded ground. The client view centers on that
actual destination, including a saved location outside the startup spawn area.
The remaining view continues to stream through the bounded background loader.

To expose both listeners from an existing custom config when starting the binary
without the preview launcher, use `--bind-all-interfaces`. It preserves Minecraft
and dashboard ports and the configured dashboard enablement. The dashboard
serves read-only HTTP routes on the selected address. If disabled, enable it with
`dashboard_enabled = true` before the `[world]` table; its default port is 9090.
