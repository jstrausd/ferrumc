# Local natural Overworld preview

The `ferrumc_natural_preview` preset connects the accepted Minecraft Java
1.21.8 / Paper build 60 numerical producers to the running Rust server:
noise materials, genuine biomes, surface rules and cold caves/ravines. It is
an explicit algorithm-2, revision-1 terrain preview, with seed 0 in the local
configuration. Ordinary new-server defaults remain `ferrumc_flat`.

## Run locally

The local delivery contains a release executable under `local-natural/`.
From the repository root:

```sh
./scripts/run-natural-preview.sh
```

Connect a **Minecraft Java 1.21.8** client to **localhost:25566**. Stop with
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

On another clean accepted checkout, first build with
`cargo build --manifest-path server/Cargo.toml --release --locked -p ferrumc-app`,
then set `FERRUMC_SERVER_BIN` to that checkout's `server/target/release/ferrumc`.
The launcher deliberately uses the supplied binary rather than compiling
unreviewed working-tree changes.

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

Player placement checks final collision shapes against live nonspectator
players before any mutation, including refined slabs and both door/bed cells.
A refusal resynchronizes the prediction and acknowledges the action. Original
offset bamboo/dripstone/moving-piston shapes remain unsupported and fail closed.

## Preview limits

- No placed-feature ore population, trees or structures. Material-stage ore
  veins remain present. An explicit optional `ferrumc_natural_grass_preview`
  preset adds original isolated plains/forest short-grass features (revision 2).
  This is a subset, not complete vegetation or full feature chronology.
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
