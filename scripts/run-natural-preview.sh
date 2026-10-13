#!/usr/bin/env bash
# Start a separate local terrain preview, preserving any existing saved world.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_DIR="${FERRUMC_NATURAL_DIR:-$ROOT/local-natural}"
if [[ -n "${FERRUMC_SERVER_BIN:-}" ]]; then
  SERVER_BIN="$FERRUMC_SERVER_BIN"
elif [[ -x "$ROOT/server/target/release/ferrumc" ]] &&
     [[ ! -x "$ROOT/local-natural/ferrumc" || "$ROOT/server/target/release/ferrumc" -nt "$ROOT/local-natural/ferrumc" ]]; then
  # A host-local rebuild must take precedence over an older delivered copy.
  SERVER_BIN="$ROOT/server/target/release/ferrumc"
else
  SERVER_BIN="$ROOT/local-natural/ferrumc"
fi
PORT="${FERRUMC_NATURAL_PORT:-25566}"
PRESET="${FERRUMC_NATURAL_PRESET:-ferrumc_natural_preview}"
case "$PRESET" in
  ferrumc_natural_preview|ferrumc_natural_grass_preview|ferrumc_natural_tree_preview) ;;
  *) echo "Unsupported preview preset: $PRESET" >&2; exit 1 ;;
esac

if (( $# > 0 )); then
  echo "This launcher accepts environment settings, not command-line arguments." >&2
  echo "It always binds to 0.0.0.0; set FERRUMC_NATURAL_PORT to select the Minecraft port." >&2
  exit 1
fi

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
bind = "0.0.0.0:$PORT"
world_dir = "world"
max_connections = 8
view_distance = 6
simulation_distance = 2
spawn_chunk_radius = 1
spawn = [8.0, 64.0, 8.0]
chunk_stream_interval_ms = 50
dashboard_enabled = false
dashboard_bind = "0.0.0.0:9090"
builtin_plugins = false

[world]
seed = 0
preset = "$PRESET"
TOML
elif [[ -n "${FERRUMC_NATURAL_PRESET:-}" ]]; then
  echo "Existing config kept: FERRUMC_NATURAL_PRESET only selects a newly created config." >&2
  echo "Check [world].preset in $RUN_DIR/config.toml; use a fresh directory to change the generator." >&2
fi
echo "Minecraft Java 1.21.8: listening on 0.0.0.0:$PORT (connect using localhost or this host’s IP)"
echo "Executable: $SERVER_BIN"
echo "Config: $RUN_DIR/config.toml; stop with Ctrl-C to save."
cd "$RUN_DIR"
exec "$SERVER_BIN" --config "$RUN_DIR/config.toml" --bind "0.0.0.0:$PORT" --bind-all-interfaces
