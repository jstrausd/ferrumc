# FerrumC — Independent Black-Box Protocol Smoke Test

A protocol gate for the FerrumC v2 Minecraft server (Java **1.21.8 / protocol
772**) driven by a client we did **not** write:
[PrismarineJS `node-minecraft-protocol`](https://github.com/PrismarineJS/node-minecraft-protocol),
whose bundled `minecraft-data` supports 1.21.8.

## Why an independent client matters (the hard lesson)

We previously gated protocol work with a **self-written fake client**. It passed.
Green across the board. Then a real vanilla Minecraft client connected and
**rejected our packets** — stuck on "Loading terrain", silent disconnects, the
works.

The reason is obvious in hindsight: when the same codebase produces both the
server's bytes *and* the test client's expectations, the test only proves the
code agrees with itself. Any shared bug — a wrong field order, a missing flag, an
off-by-one VarInt — is invisible, because both sides make the identical mistake.
That is not a test; it is a mirror. It manufactures **false confidence**.

`node-minecraft-protocol` is a third-party implementation maintained by a
different community, with its own independent decode/encode of the 1.21.8 wire
format (via `minecraft-data`). If FerrumC's bytes are wrong, this client
complains — exactly like a real player's client would. That is the entire point:
**the test oracle must be independent of the system under test.**

The same independence applies to **block state**: placed-block state ids are
decoded with `prismarine-block`, and streamed chunks are parsed with
`prismarine-chunk` — both third-party PrismarineJS libraries. So when the test
asserts "the log the server placed has `axis=x`" or "the edit is still there
after a restart", that fact is read back through a decoder we did not write.

## What it checks

The smoke (`smoke.mjs`) runs the full alpha flow end to end:

1. **Status ping** — asserts the server reports `protocol === 772` and a version
   string containing `1.21.8`.
2. **Offline-mode login** — completes handshake -> login -> configuration and
   reaches the **`play`** state (join-game received).
3. **First chunk(s)** — waits for the first chunk column(s) to stream in.
4. **Movement across a chunk boundary** — walks in small, server-acceptable
   steps and asserts it observes **both** new chunk **loads** (leading edge) and
   chunk **unloads** (trailing edge).
5. **Creative set-slot** — `set_creative_slot` to put `oak_log` / `oak_slab` /
   `oak_stairs` into the hotbar.
6. **Stateful block placement** — places blocks whose **block-state depends on
   how you place them**, and asserts the `block_change` + sequence
   `acknowledge_player_digging` the server returns carry the expected state id:
   - **log** — `axis` from the clicked face (east face -> `axis=x`, state 136),
   - **slab** — `type` from the cursor height on a side face (`cursorY=0.9` ->
     `type=top`, state 12052),
   - **stairs** — `facing` from the player's yaw (`yaw=0` -> `facing=south`),
     `half` from the cursor (`half=bottom`, state 2969).
7. **Block break** — `block_dig` (creative insta-mine) and asserts the
   `block_change` to **air** + the sequence ack.
8. **Cross-chunk edit** — walks into a non-spawn chunk, places a block, confirms.
9. **Disconnect + reconnect** — rejoins and verifies every placed/broken block
   streams back in (read out of the parsed chunks).
10. **Restart persistence (the kill-shot)** — **stops the server process**,
    starts it again on the **same world dir**, reconnects, and verifies the whole
    pattern survived — i.e. it was flushed to redb and read back, not just held
    in memory. *(Managed mode only — see below.)*

The placement matrix also uses actual torch, soul-torch and redstone-torch
items on real stone supports (floor and four walls), and edits beside an
existing oak fence. Fence cases cover other wood including crimson, nether
brick, both gate axes, leaves/pumpkin exceptions and glass. A later World-FIFO
marker proves unchanged secondary states without relying on ACK priority or
timeouts. Rejoin and restart read the final pattern through Prismarine chunks.
Support loss, general collision/support shapes, fluids and redstone remain open.

It exits `0` on success and non-zero with a clear diagnostic dump on failure.
Along the way it does the housekeeping a real client must do to stay connected:
replies to keep-alives and confirms server teleports.

## Two modes

- **Managed** (`MC_MANAGE_SERVER=1`, the `run-smoke.sh` default): the script owns
  the server lifecycle. It spawns the **built binary directly** (not `cargo run`,
  so a `SIGINT` reaches the server and triggers its graceful flush-on-shutdown),
  runs steps 1–9, then **stops and restarts** the process for step 10. Uses a
  fresh world under a temp dir, on port `25599` by default.
- **External** (`MC_MANAGE_SERVER=0`, the default when you run `node smoke.mjs`
  by hand): runs steps 1–9 against a server **you** started, on port `25565` by
  default. Step 10 is **skipped** — we can't restart a process we don't own.

> **Stop signal:** FerrumC handles **SIGINT**, **SIGTERM**, and **SIGHUP** for
> graceful shutdown, including flush of pending edits. The managed restart uses
> SIGINT. SIGKILL cannot run shutdown handlers. A hard-kill (SIGKILL) crash-
> durability check is **not automated** — it depends on redb commit timing and
> would be flaky; verify it by hand if you need that guarantee.

## Running it locally

Prereqs: Node >= 18, `pnpm`, and (for managed mode) a Rust toolchain.

```bash
cd tests/blackbox

# Managed (default): builds the server, then spawns/stops/restarts it and runs
# the full scenario including restart-persistence.
./run-smoke.sh

# External: start a server yourself, then point the smoke at it.
cargo run                                   # from the server/ workspace
MC_MANAGE_SERVER=0 ./run-smoke.sh 127.0.0.1 25565
# or directly:
MC_MANAGE_SERVER=0 node smoke.mjs 127.0.0.1 25565
```

Useful env knobs: `MC_HOST`, `MC_PORT`, `MC_USERNAME`, `MC_MOVE_BLOCKS`,
`MC_STEP_BLOCKS`, `MC_STEP_MS`, `MC_SERVER_BIN` (managed: path to the server
binary), `MC_RUN_DIR` (managed: where the temp world/config/logs go),
`MC_KEEP_RUN=1` (managed: don't wipe the run dir at start), `RUST_LOG`.

Every step prints a `[PASS]` / `[FAIL]` / `[SKIP]` line; a managed run's server
logs land in `$MC_RUN_DIR/server-*.log` (default under the OS temp dir).

## Status

| Step                              | State        |
|-----------------------------------|--------------|
| status ping (proto/version)       | implemented  |
| offline login -> play             | implemented  |
| first chunk(s)                    | implemented  |
| move across chunk boundary        | implemented  |
| chunk load + unload observed      | implemented  |
| set creative-mode slot            | implemented  |
| select hotbar slot                | implemented  |
| place stateful block (log/slab/stairs) | implemented |
| assert placed block-state id      | implemented  |
| break block (-> air) + ack        | implemented  |
| cross-chunk edit                  | implemented  |
| disconnect / reconnect            | implemented  |
| verify persistence after restart  | implemented (managed mode) |

## Files

- `package.json` — pins `minecraft-protocol` + `minecraft-data` (the wire client)
  and `prismarine-registry` / `prismarine-chunk` / `prismarine-block` / `vec3`
  (the independent block-state + chunk oracle). `type: module`.
- `smoke.mjs` — the smoke test (full scenario + managed server lifecycle).
- `run-smoke.sh` — runner; managed by default (builds the server, runs the full
  scenario incl. restart); `MC_MANAGE_SERVER=0` for an external server.
- `README.md` — this file.

## Live operator commands and local recovery

`operator-commands.mjs` owns a native-only Rust process and three sequential runs
on the same fresh world. Run it after building the server:

```sh
MC_SERVER_BIN=/path/to/ferrumc node operator-commands.mjs
```

Independent clients decode command graphs and actual game-mode events. They
verify `/op` and `/deop`, immediate permission revocation, a grant to an offline
name, and a durable revocation of a configured seed after process restart.
The restarted server accepts `op`/`deop` through its actual stdin pipe, refreshes
the connected clients, and remains usable after stdin EOF. Both runs must exit
cleanly after SIGTERM; the third run reloads the console's grant and revocation.
Malformed records, write failures, cancellation and
shutdown-admission contracts are separate Rust tests. Signed commands, vanilla
`ops.json`, selectors, client permission-status EntityEvents, Java runtime
mutation and full Paper command behavior are outside this gate.

## Java plugin client contract

`java-plugin.mjs` combines the independent Minecraft protocol decoder with the
ordinary Bukkit JAR built in `server/java-bridge/fixture`. Build the server and
run Maven `verify` in `server/java-bridge/`, then run:

```sh
FERRUMC_JAVA_EXECUTABLE=/path/to/java21/bin/java \
FERRUMC_JAVA_BRIDGE_JAR=/path/to/ferrumc-java-bridge-0.1.0.jar \
FERRUMC_JAVA_FIXTURE_JAR=/path/to/standard-bukkit-fixture-0.1.0.jar \
MC_SERVER_BIN=/path/to/ferrumc node java-plugin.mjs
```

The test creates a fresh temporary data directory and manages its Rust server.
Missing artifacts fail the test. Two clients verify ordered join listeners,
command graph visibility, aliases, canonical UUIDs, UTF-8 messages, permissions,
literal completion, explicit unsupported API errors and message routing. Graceful
stop must invoke `onDisable` once and persist the fixture's config. This proves
the implemented subset; the fixture is not an external plugin compatibility
matrix or a full Paper acceptance test.

Minecraft 1.21.8 NBT strings use Java `DataOutput.writeUTF` / `DataInput.readUTF`
(Modified UTF-8). The bundled PrismarineJS NBT decoder uses ordinary UTF-8 for
non-BMP characters and displays six replacement characters for an emoji's
surrogate pair. The client text assertion therefore uses `Grüße Ω`; subprocess
contracts separately verify emoji text over the bridge's ordinary UTF-8 JSON.
This decoder limitation must not prompt a change to Minecraft's NBT encoding.

For a separately managed fixture server, set `MC_JAVA_EXTERNAL=1`, `MC_HOST`,
and `MC_PORT`. That mode runs the two-client checks; the operator or container
harness must separately check shutdown and persisted lifecycle counters.

## Full-bag inventory cursor regression (I01)

`inventory-cursor.mjs` always manages a built server on a free loopback port with
a fresh temporary world. It requires the dependencies already pinned here:

```sh
MC_SERVER_BIN=/path/to/ferrumc node inventory-cursor.mjs
```

It fills all 36 main/hotbar slots with 64 stone each, picks 13 diamonds out of a
chest, and checks the authoritative cursor and total counts after close/reopen,
switching chests, delayed repeated closes, a stale click, disconnect/rejoin and
a full process restart with a chest still open. A concurrent connection for the
same offline UUID must be explicitly rejected while the original cursor remains
unchanged. An immediate rejoin may retry only the explicit transient UUID-reserved
reason, within a bounded admission budget; other join failures fail the gate.
It then deposits exactly 13
diamonds into the second chest, verifies the first chest is empty, and restarts
again to verify that deposit persisted without duplication. Every action waits
for its inventory snapshot, slot echo, sequence acknowledgement or process exit;
there are no timing-based absence assertions. Any failed mandatory step exits
nonzero and leaves the fresh logs/world for diagnosis; cleanup reaps the owned
server, including a bounded forced-stop fallback.

A separate live check verifies a custom-named stack survives a full-bag close,
reopen and chest switch. The schema-3 restart scenario preserves custom names,
damage and component removals in player slots, armor, offhand and a full-bag
cursor through rejoin and two process restarts. The script manages six server
processes in total. Chest exchanges use normal left-clicks; this does not imply
other chest click modes or full vanilla inventory support. External-server mode
is intentionally absent because process restarts are required assertions.

## Chest component persistence (S02)

`MC_SERVER_BIN=/path/to/ferrumc node chest-component-persistence.mjs` manages
a fresh world and three server processes. It deposits and retrieves an item
through survival chest clicks across two real process restarts, checks total
counts and compares exact Slot bytes using the independent Prismarine parser
for field boundaries. The marker includes a custom name, damage, a removed
component, duplicate NBT keys, NaN payload bits and signed zero. Creative mode
only creates the fixture; all chest transfers run in survival mode. This proves
container persistence for the tested stack, not complete survival gameplay or
hard-kill atomicity across player and world records.

## Player inventory normal clicks (I03)

`MC_SERVER_BIN=/path/to/ferrumc node player-inventory.mjs` manages a fresh server
with independent PrismarineJS clients. It checks named-stack split/merge/swap,
place-one, offhand equipment broadcast, stale/unsupported actions, spectator
refusal and conservative close behavior with full storage and crafting inputs.
The client-supplied slot deltas are ignored in favor of server-owned state.
This gate does not establish recipes, armor insertion or other click modes.
