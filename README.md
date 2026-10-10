# FerrumC

A high-performance Minecraft Java Edition server implementation written in Rust.

> **v2 development branch.** This checkout starts from upstream
> `rework/v2-skeleton`. The code is ahead of several older feature documents;
> see the [audited implementation plan](docs/V2_IMPLEMENTATION_PLAN.md) and
> [validation report](docs/V2_VALIDATION.md) for this fork's current work.

## Status: v2 rework (creative/minigame core)

FerrumC v2 is a **deterministic, Rust-native, observable creative/minigame server core** for vanilla **Minecraft: Java Edition 1.21.8** clients. It is a clean-room core meant for creative and minigame servers — not a drop-in vanilla replacement.

It is **not** vanilla parity, **not** survival, and **not** a Paper/Spigot replacement. Scope today: **flat generation or a limited Anvil map import**, **offline/local alpha**. The client target is Minecraft: Java Edition 1.21.8 (protocol 772) — see [`SUPPORTED_VERSION.md`](SUPPORTED_VERSION.md).

> **NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.**

## FerrumC v2 (rework branch) — current status

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
- Player position, game mode, hotbar and inventory are loaded/saved on join/leave/shutdown; item components are currently omitted.
- Limited startup import of Anvil blocks, plain signs and empty chests.
- Built-in and **trusted native plugins** with events, mutation intents and block-edit decisions through a versioned C ABI. Dynamic bundles are target-specific and require a manifest and pinned library hash; additional host facades remain unfinished.
- A read-only observability dashboard at `http://127.0.0.1:9090` (loopback-only by design).

**Limitations (not done yet):**

- Flat terrain generation; Anvil import has a limited content contract, and export is unfinished.
- Offline mode only — no online-mode authentication/encryption.
- Full-bright lighting (placeholder, no light engine).
- No survival, mobs, redstone, or full vanilla parity.
- Player inventory clicks, cursor conservation and component persistence need further work.

**Roadmap:** reliable hosting and connection handling, state correctness,
online-mode authentication, fuller plugin facades, world import/export, and
lighting. See the [prioritized plan](docs/V2_IMPLEMENTATION_PLAN.md).

See `docs/architecture/overview.md` for the design, [`docs/public-alpha.md`](docs/public-alpha.md) for the public-alpha checklist, and `CLAUDE.md` for development instructions.

## Building

```bash
cd server
cargo build --release --locked -p ferrumc-app
./target/release/ferrumc --help
```

The default Minecraft bind is loopback. To accept clients from outside the host,
start with `--bind 0.0.0.0:25565`. Config and the persistent `world/` directory
are created relative to the working directory. Container and Pterodactyl setup
are documented in [deploy/README.md](deploy/README.md).

## License

MIT — see [LICENSE](LICENSE).

## Links

- **Website:** [ferrumc.com](https://ferrumc.com)
- **Discord:** [Join](https://discord.gg/ferrumc)
- **GitHub:** [ferrumc-rs/ferrumc](https://github.com/ferrumc-rs/ferrumc)
