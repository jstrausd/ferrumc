# Local natural Overworld preview

The `ferrumc_natural_preview` preset connects the accepted Minecraft Java
1.21.8 / Paper build 60 numerical producers to the running Rust server:
noise materials, genuine biomes, surface rules and cold caves/ravines. It is
an explicit algorithm-2, revision-1 terrain preview, with seed 0 in the local
configuration. Ordinary new-server defaults remain `ferrumc_flat`.

## Run locally

The launcher selects the newer executable from `local-natural/ferrumc` and
`server/target/release/ferrumc`, or the only available one. An explicit
`FERRUMC_SERVER_BIN` overrides this selection. It prints the selected path
before starting. From the repository root:

```sh
./scripts/run-natural-preview.sh
```

Connect a **Minecraft Java 1.21.8** client to **localhost:25566**, or this host's
address from another machine. Minecraft and the enabled web dashboard bind to
`0.0.0.0`; the launcher enforces this for existing configs too. Stop with
Ctrl-C to flush world/player changes before restarting. If the delivered
process is already running, connect directly; a second process cannot open
the same world database. Its log is `local-natural/server.log`.

The launcher creates `local-natural/config.toml` only if absent and keeps
the save in `local-natural/world/`. It does not replace an existing server
world. Edit the local config while stopped. Natural spawn Y is selected on
a dry surface in the configured spawn ring; configured X/Z guide the search.
An all-water ring fails startup with an actionable error.

For another accepted release executable or a separate test directory:

```sh
FERRUMC_SERVER_BIN=/absolute/path/to/ferrumc \
FERRUMC_NATURAL_DIR=/absolute/path/to/fresh-preview \
FERRUMC_NATURAL_PORT=25568 ./scripts/run-natural-preview.sh
```

On another clean accepted checkout:

```sh
git pull --ff-only
cargo build --manifest-path server/Cargo.toml --release --locked -p ferrumc-app
FERRUMC_SERVER_BIN="$PWD/server/target/release/ferrumc" ./scripts/run-natural-preview.sh
```

Building updates `server/target/release/ferrumc`; it does not replace a previously
installed `local-natural/ferrumc`. The explicit path above also works with older
launchers. For a custom Cargo target directory, set `FERRUMC_SERVER_BIN` to its
actual executable. The launcher uses existing binaries and never builds source.
Do not append `--bind`: both listeners already bind to `0.0.0.0`. Select a
different Minecraft port with `FERRUMC_NATURAL_PORT`.

## Configuration and persistence

Set all top-level settings before the `[world]` table:

```toml
world_dir = "fresh-natural-world"
spawn_chunk_radius = 1
view_distance = 6
chunk_stream_interval_ms = 50

[world]
seed = 0
preset = "ferrumc_natural_preview"
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
./scripts/run-natural-preview.sh
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
./scripts/run-natural-preview.sh
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
RUST_BACKTRACE=1 RUST_LOG=info \
FERRUMC_SERVER_BIN="$PWD/server/target/release/ferrumc" \
FERRUMC_NATURAL_DIR="$PWD/local-natural/tree-preview-v3" \
FERRUMC_NATURAL_PRESET=ferrumc_natural_tree_preview \
./scripts/run-natural-preview.sh 2>&1 | tee tree-preview.log
```

This keeps existing saves and configs. A preset environment variable cannot
upgrade a previously generated world; the launcher warns when reusing its config.

Player placement checks final collision shapes against live nonspectator
players before any mutation, including refined slabs and both door/bed cells.
A refusal resynchronizes the prediction and acknowledges the action. Original
offset bamboo/dripstone/moving-piston shapes remain unsupported and fail closed.

## Preview limits

- No complete placed-feature ore population or structures. Material-stage ore
  veins remain present. An explicit optional `ferrumc_natural_grass_preview`
  preset adds original isolated plains/forest short-grass features (revision 2).
  This is a subset, not complete vegetation or full feature chronology.
- The optional `ferrumc_natural_tree_preview` adds undecorated configured oak
  and birch shapes. It omits fancy oak, fallen trees, bee nests, leaf litter and
  other tree species. Its isolated-source placement policy is versioned and
  does not reproduce the complete original FEATURES stage.
- Overworld only. Complete Survival, mobs, Nether, End and the dragon remain
  separate work.
- Lighting is direct vertical skylight; lateral propagation and emitted block
  light are incomplete. Heightmaps use the current highest-non-air model.
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

The 13 October loading optimization keeps the same revision-3 tree world and
32/64/32 raw/biome/source cache limits. Existing `tree-preview` saves can be
restarted with the updated executable and the same launch command above.

In the standard seed-0 protocol test, all 169 view-6 columns loaded in
**129.838 seconds** initially and **125.120 seconds** after restart, versus
173.439/172.998 seconds in the previous standard run (25.14%/27.68% less time).
Complete generated block/biome snapshots and saved edits stayed identical.
These are individual same-host full-view observations, not general TPS/RSS or
Paper-server benchmarks. Initial loading remains slow and other species,
decorators, leaf decay and full Survival remain open.

See [the exact validation and evidence](TREE_GENERATION_VALIDATION.md#loading-optimization-13-october-2026).

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
