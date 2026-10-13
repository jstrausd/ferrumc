#!/usr/bin/env bash
# Start a separate local terrain preview, preserving any existing saved world.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_DIR="${FERRUMC_NATURAL_DIR:-$ROOT/local-natural}"
SERVER_BIN="${FERRUMC_SERVER_BIN:-$ROOT/local-natural/ferrumc}"
PORT="${FERRUMC_NATURAL_PORT:-25566}"
PRESET="${FERRUMC_NATURAL_PRESET:-ferrumc_natural_preview}"
case "$PRESET" in
  ferrumc_natural_preview|ferrumc_natural_grass_preview) ;;
  *) echo "Unsupported preview preset: $PRESET" >&2; exit 1 ;;
esac

if [[ ! -x "$SERVER_BIN" ]]; then
  echo "Missing executable: $SERVER_BIN" >&2
  echo "Build an accepted checkout with cargo build --manifest-path server/Cargo.toml --release --locked -p ferrumc-app" >&2
  echo "Then set FERRUMC_SERVER_BIN to the absolute server/target/release/ferrumc path." >&2
  exit 1
fi
SERVER_BIN="$(realpath "$SERVER_BIN")"
mkdir -p "$RUN_DIR"
RUN_DIR="$(cd "$RUN_DIR" && pwd)"
if [[ ! -f "$RUN_DIR/config.toml" ]]; then
  cat > "$RUN_DIR/config.toml" <<TOML
# Local Minecraft Java 1.21.8 natural terrain preview.
# World data is separate from the ordinary server's world directory.
bind = "127.0.0.1:25566"
world_dir = "world"
max_connections = 8
view_distance = 6
simulation_distance = 2
spawn_chunk_radius = 1
spawn = [8.0, 64.0, 8.0]
chunk_stream_interval_ms = 50
dashboard_enabled = false
builtin_plugins = false

[world]
seed = 0
preset = "$PRESET"
TOML
fi
echo "Minecraft Java 1.21.8: localhost:$PORT"
echo "Config: $RUN_DIR/config.toml; stop with Ctrl-C to save."
cd "$RUN_DIR"
exec "$SERVER_BIN" --config "$RUN_DIR/config.toml" --bind "127.0.0.1:$PORT"
