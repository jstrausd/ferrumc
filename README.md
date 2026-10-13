# FerrumC

A high-performance Minecraft Java Edition server implementation written in Rust.

> **v2 on `main`.** This fork has one working branch. Its starting point was
> upstream `rework/v2-skeleton`. The code is ahead of several older feature documents;
> see the [audited implementation plan](docs/V2_IMPLEMENTATION_PLAN.md) and
> [validation report](docs/V2_VALIDATION.md) for this fork's current work.

## Status: v2 Rust core and a tested Paper 1.21.8 API subset

FerrumC v2 provides a deterministic Rust creative/minigame core for
**Minecraft: Java Edition 1.21.8** clients. This fork is expanding that core
toward complete Survival, low memory/CPU use and unmodified Java plugins. Rust owns the world
and networking; an optional Java 21 adapter implements the first tested Paper
API subset.

Current gameplay uses flat generation, a versioned natural Overworld preview,
or a limited Anvil map import in an offline alpha. Survival, online authentication and the remaining Paper APIs
remain unfinished. The client protocol is 772; see
[`SUPPORTED_VERSION.md`](SUPPORTED_VERSION.md).

> **NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.**

## FerrumC v2 on main — current status

A creative/minigame development core targeting **Minecraft Java 1.21.8
(protocol 772)**, **offline mode only**. The older `docs/FEATURES.md` and
`docs/ROADMAP.md` are historical checklists; use the
[source audit and backlog](docs/V2_IMPLEMENTATION_PLAN.md) for current scope.

**What works today:**

- Join a vanilla 1.21.8 client in offline mode (no "Loading terrain" hang).
- A flat creative world; build by placing blocks from the hotbar with **correct block states** — logs/slabs/stairs/torches/fences rotate and orient the way a real client expects.
- Optional natural Overworld terrain, biomes and caves, with separately versioned grass and oak/birch tree subsets. See [local preview commands](docs/LOCAL_NATURAL_WORLD.md) and the [tree validation scope](docs/TREE_GENERATION_VALIDATION.md).
- Multiplayer: other players are visible and move, **face the right way** (body + head rotation), and you can **see their held main-hand item**.
- In-game chat and `/` command autocomplete (`/spawn`, `/gamemode`, permission-filtered).
- Server-authoritative normal left/right player-inventory clicks with component-preserving split/merge/swap, offhand updates and cursor resync. Spectators cannot exchange inventory or chest items. Recipes and additional click modes remain unfinished.
- Absolute pre-Play phase and first-byte frame deadlines; ignored traffic and partial byte drips cannot renew them.
- Persistence across **leave/rejoin and restart** — placed blocks stick.
- Player position, game mode, hotbar, all 46 inventory slots and the retained full-bag cursor are loaded/saved on join/leave/shutdown. Schema 3 preserves supported item components, opaque payloads, removals and NBT details; JSON schemas 1/2 migrate explicitly without rewriting on load.
- Checked chest/overlay persistence preserves supported components across reloads. Invalid or oversized contents reject the complete capture/load. Dirty ownership transfers only after successful capture and queue admission; last chunk tickets release only after the durability barrier succeeds.
- Limited startup import of Anvil blocks, plain signs and empty chests.
- Built-in and **trusted native plugins** with events, mutation intents and block-edit decisions through a versioned C ABI. Dynamic bundles are target-specific and require a manifest and pinned library hash; additional host facades remain unfinished.
- Optional ordinary Bukkit `plugin.yml` JARs: lifecycle and configuration, declared commands/aliases, plugin argument completion, current-player identity/messages, Rust command permissions, and join/quit listeners. Unsupported Java APIs report concrete errors; see the [tested API contract](server/java-bridge/README.md).
- A read-only observability dashboard on `0.0.0.0:9090` (open locally at `http://localhost:9090`).

**Limitations (not done yet):**

- Natural generation is an Overworld preview: complete placed ores, structures and vegetation remain unfinished. The tree subset omits other species, fancy/fallen trees and decorators. Anvil import has a limited content contract, and export is unfinished.
- Offline mode only — no online-mode authentication/encryption.
- Flat worlds use full-bright lighting; natural worlds have vertical skylight only. A complete light engine remains unfinished.
- No survival, mobs, redstone, or full vanilla parity.
- Recipes, armor insertion rules, shift/drag/number-key/drop clicks need further work. Full-bag crafting input overflow stays in its input slot until ground-item entities exist. Components omitted by older saves cannot be recovered. Atomic crash recovery across player, chest/overlay and journal records remains unfinished; the experimental scheduler still uses the legacy persistence path.
- Java world/entity/inventory APIs, scheduler, full Bukkit permissions, Paper bootstrap and NMS integration need further work.

**Roadmap:** build the remaining Survival systems on the inventory/component
foundation: ground items, mining/loot, recipes, health/hunger/combat, entities,
world ticks and lighting. Online authentication and Java world/inventory/event
facades follow their corresponding Rust behavior. See the
[prioritized plan](docs/V2_IMPLEMENTATION_PLAN.md).

Resource measurements use the [process baseline harness](tests/performance/README.md)
to record Rust and the optional JVM separately. The frozen Release
[`7333f404` baseline](tests/performance/results/2026-10-10/README.md) measured
21.48–21.70 MiB Rust RSS with eight standing clients; Rust plus the Java host and
Bukkit fixture used 108.41–108.72 MiB. These runs exercised no active gameplay
and predate the current S02 changes. Rust can offer resource advantages, but a
Paper comparison requires equivalent gameplay, worlds, plugins and load.

See `docs/architecture/overview.md` for the design, [`docs/public-alpha.md`](docs/public-alpha.md) for the public-alpha checklist, and `CLAUDE.md` for development instructions.

## Building

Local development, CI, and Docker use Rust **1.99.0**, selected by
`rust-toolchain.toml`. Older Rust releases are not supported by this fork.

```bash
cd server
cargo build --release --locked -p ferrumc-app
./target/release/ferrumc --help
```

Minecraft and the read-only web dashboard bind to `0.0.0.0` by default.
Use `--bind-all-interfaces` to apply this to both listeners when an existing
config names a specific address, preserving the configured ports. Config and the persistent `world/` directory
are created relative to the working directory. Container and Pterodactyl setup
are documented in [deploy/README.md](deploy/README.md).

## Java plugin expansion

Unmodified Bukkit/Paper `.jar` plugins are a requested development target.
The Rust simulation remains authoritative. A Java sidecar using the real
Paper 1.21.8 API now runs the tested subset described above. A separately built
ordinary Bukkit fixture has passed Java-process tests, Rust socket tests and
an independent two-client protocol test. That fixture establishes the listed
API behavior; broad external plugin acceptance remains open.

Build the adapter with Java 21 and Maven (`cd server/java-bridge && mvn -B verify`).
The [Docker/Pterodactyl image](deploy/README.md) includes the adapter, Java runtime
and corresponding sources. Enable it through the two top-level paths in
`config.toml`; installation instructions and exact limits are in the
[adapter README](server/java-bridge/README.md). The remaining contracts are in
[ADR-0009](docs/adr/0009-java-plugin-compatibility.md).

## License

MIT — see [LICENSE](LICENSE).

## Links

- **Website:** [ferrumc.com](https://ferrumc.com)
- **Discord:** [Join](https://discord.gg/ferrumc)
- **GitHub:** [ferrumc-rs/ferrumc](https://github.com/ferrumc-rs/ferrumc)
