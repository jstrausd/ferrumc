# FerrumC v2 on Docker and Pterodactyl

This deployment builds the current Rust v2 server for Minecraft Java 1.21.8
(protocol 772). The server currently runs as an offline alpha with a flat-world
default, optional Anvil import, native FerrumC plugins, and placeholder lighting.
Use a private or otherwise controlled network while online authentication is
being implemented; offline player names are not authenticated identities.

## Build and run locally

Run from the repository root:

```sh
docker build -t mchosting/ferrumc:v2-local .
sudo install -d -m 0750 -o 1000 -g 1000 /srv/ferrumc-v2
docker run --detach --name ferrumc-v2 \
  --stop-timeout 60 \
  --publish 25565:25565/tcp \
  --env SERVER_PORT=25565 \
  --mount type=bind,src=/srv/ferrumc-v2,dst=/home/container \
  mchosting/ferrumc:v2-local
docker logs --follow ferrumc-v2
```

The image builds `server/` with Rust 1.99.0 using
`cargo build --release --locked -p ferrumc-app`. The Debian bookworm runtime
matches the build stage's glibc. The process runs as UID/GID 1000, so a bind
mount must be writable by that user. First startup creates `config.toml` and
the durable `world/` database in the mounted directory.

`SERVER_PORT` selects the listener port and accepts decimal integers in
`1..65535`; omitted or empty values use 25565. Match the Docker published port
to this value. The entrypoint always supplies `--bind 0.0.0.0:<SERVER_PORT>`
so the listener is reachable through the container network.

Docker arguments replace the default `--config /home/container/config.toml`
arguments and are passed literally to FerrumC. For example, append
`--config /home/container/custom.toml` after the image name to use another
config, or append `--help` to display the CLI. Shell expressions are not
expanded by the entrypoint. Avoid an additional `--bind` argument because the
entrypoint already supplies it; `--port` overrides the listener port through
the application's CLI and must match the port mapping.

The image keeps `/usr/local/bin/ferrumc` and
`/usr/local/share/ferrumc/config.template.toml` outside the data mount. Rebuild
the image to update the binary, then recreate the container with the same data
mount. Stop gracefully with `docker stop --time 60 ferrumc-v2`; FerrumC receives
SIGTERM directly as PID 1 and flushes on shutdown. Wait for the
`shutdown complete` log before taking a filesystem backup of `world/` and keep
`config.toml` and plugin bundles with that backup. A forced kill can lose
unflushed changes and connected-player state.

## Pterodactyl installation

Build both images from the repository root:

```sh
docker build --target installer -t mchosting/ferrumc-installer:v2-local .
docker build -t mchosting/ferrumc:v2-local .
```

The final default Dockerfile stage is `runtime`. The separate `installer` stage
runs as root, has no entrypoint, and carries the config template. Wings supplies
the command `/bin/sh /mnt/install/install.sh` during installation and preserves
the image's user and entrypoint; this separate image lets that shell run directly.

1. Make both `mchosting/ferrumc:v2-local` and
   `mchosting/ferrumc-installer:v2-local` available in the Docker image store of
   each Wings node. Build this checkout on that node or transfer both images with
   `docker save` / `docker load`.
2. Import `deploy/pterodactyl/egg-ferrumc-v2.json` in the panel and choose its
   `mchosting/ferrumc:v2-local` image. The egg references a local image; there is
   no published image or release download configured. If the node's image policy
   requires registry pulls, configure a registry you control and update the egg's
   runtime and installation references to their respective image tags.
3. Allocate a TCP port and ensure the node's container user is UID/GID 1000.
   Wings provides the allocation as `SERVER_PORT`; the entrypoint binds that
   port without rewriting the TOML file.
4. Run installation and start the server. Installation uses the separate
   `mchosting/ferrumc-installer:v2-local` image. It creates absent `world/` and `plugins/` directories
   and copies the image's commented template to `/mnt/server/config.toml` only
   when no config exists. Reinstallation preserves existing file contents.

The egg's displayed startup command describes the image's fixed invocation:

```text
/usr/local/bin/ferrumc --bind 0.0.0.0:{{SERVER_PORT}} --config /home/container/config.toml
```

The image does not execute Wings' `STARTUP` environment variable. Editing the
panel startup text therefore does not change the command. Configure the server
through `config.toml`, the allocated port, and `RUST_LOG`; build a derived image
or deliberately replace the entrypoint for a different startup contract. The
egg performs no port substitutions or executable downloads.

Readiness matches `listening for Minecraft clients`. The panel stop value `^C`
sends SIGINT; FerrumC also handles SIGTERM. Allow at least 60 seconds before a
forced stop so connections, player saves, and the storage worker can drain.
Pterodactyl's console displays logs; administrative commands are available
in-game according to permissions.

## Persistent configuration and plugins

Keep `config.toml`, `world/`, and `plugins/` under `/home/container`. Relative
paths resolve from that directory. For custom native bundles, uncomment
`plugins_dir = "plugins"` in the template's top-level storage/plugins section
and follow [`docs/plugin-authoring.md`](../docs/plugin-authoring.md). A bundle
contains its manifest and hash-pinned Linux dynamic library. Compile it for the
node's CPU architecture and a compatible glibc, using the project's plugin ABI.
Add any extra shared libraries it requires in a derived runtime image. Plugins
execute trusted native code with the server process's privileges.

The dashboard stays at `127.0.0.1:9090` inside the container and is not published
by the example or the egg. Access it through a controlled tunnel/proxy when
needed; its runtime requires a loopback bind.
