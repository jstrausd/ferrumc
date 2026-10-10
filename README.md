# FerrumC

A high-performance Minecraft Java Edition server implementation written in Rust.

> **v2 on `main`.** This fork has one working branch. Its starting point was
> upstream `rework/v2-skeleton`. The code is ahead of several older feature documents;
> see the [audited implementation plan](docs/V2_IMPLEMENTATION_PLAN.md) and
> [validation report](docs/V2_VALIDATION.md) for this fork's current work.

## Status: v2 Rust core and a tested Paper 1.21.8 API subset

FerrumC v2 provides a deterministic Rust creative/minigame core for
**Minecraft: Java Edition 1.21.8** clients. This fork is expanding that core
toward normal server behavior and unmodified Java plugins. Rust owns the world
and networking; an optional Java 21 adapter implements the first tested Paper
API subset.

Current gameplay uses flat generation or a limited Anvil map import in an
offline alpha. Survival, online authentication and the remaining Paper APIs
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
- Multiplayer: other players are visible and move, **face the right way** (body + head rotation), and you can **see their held main-hand item**.
- In-game chat and `/` command autocomplete (`/spawn`, `/gamemode`, permission-filtered).
- Persistence across **leave/rejoin and restart** — placed blocks stick.
- Player position, game mode, hotbar, inventory and retained full-bag cursor are loaded/saved on join/leave/shutdown. Schema 1 migrates to schema 2; item components are currently omitted from persistence.
- Limited startup import of Anvil blocks, plain signs and empty chests.
- Built-in and **trusted native plugins** with events, mutation intents and block-edit decisions through a versioned C ABI. Dynamic bundles are target-specific and require a manifest and pinned library hash; additional host facades remain unfinished.
- Optional ordinary Bukkit `plugin.yml` JARs: lifecycle and configuration, declared commands/aliases, plugin argument completion, current-player identity/messages, Rust command permissions, and join/quit listeners. Unsupported Java APIs report concrete errors; see the [tested API contract](server/java-bridge/README.md).
- A read-only observability dashboard at `http://127.0.0.1:9090` (loopback-only by design).

**Limitations (not done yet):**

- Flat terrain generation; Anvil import has a limited content contract, and export is unfinished.
- Offline mode only — no online-mode authentication/encryption.
- Full-bright lighting (placeholder, no light engine).
- No survival, mobs, redstone, or full vanilla parity.
- Player inventory clicks and component persistence need further work. Cursor conservation is tested for normal closes, rejoin and successful graceful saves; player/chest crash-atomicity remains unfinished.
- Java world/entity/inventory APIs, scheduler, full Bukkit permissions, Paper bootstrap and NMS integration need further work.

**Roadmap:** reliable hosting and connection handling, state correctness,
online-mode authentication, fuller plugin facades, world import/export, and
lighting. See the [prioritized plan](docs/V2_IMPLEMENTATION_PLAN.md).

See `docs/architecture/overview.md` for the design, [`docs/public-alpha.md`](docs/public-alpha.md) for the public-alpha checklist, and `CLAUDE.md` for development instructions.

## Building

Local development, CI, and Docker use Rust **1.99.0**, selected by
`rust-toolchain.toml`. Older Rust releases are not supported by this fork.

```bash
cd server
cargo build --release --locked -p ferrumc-app
./target/release/ferrumc --help
```

The default Minecraft bind is loopback. To accept clients from outside the host,
start with `--bind 0.0.0.0:25565`. Config and the persistent `world/` directory
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
