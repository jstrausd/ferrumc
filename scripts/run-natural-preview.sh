#!/usr/bin/env bash
# Start local Survival with the latest supported natural generator.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LATEST_PRESET="ferrumc_natural_playable"
if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  cat <<HELP
Usage: ./start.sh

Starts Minecraft Java 1.21.8 Survival on 0.0.0.0:25566 with debug logging.
Fresh saves use $LATEST_PRESET (terrain, grass, trees, ores and the
implemented structure subset). The normal save is local-play/world.
Existing saves keep their original generator and player edits.

Optional environment settings:
  RUST_LOG                    Logging filter (default: debug)
  FERRUMC_SERVER_BIN           Explicit executable
  FERRUMC_NATURAL_DIR          Separate save/config directory
  FERRUMC_NATURAL_PORT         Minecraft port (default: 25566)
  FERRUMC_NATURAL_PRESET       Generator for a NEW config only

Build after updating source:
  cargo build --manifest-path server/Cargo.toml --release --locked -p ferrumc-app
Stop with Ctrl-C and wait for the save to finish before restarting.
HELP
  exit 0
fi
if (( $# > 0 )); then
  echo "No arguments are needed. Use --help for optional environment settings." >&2
  exit 1
fi

RUN_DIR="${FERRUMC_NATURAL_DIR:-$ROOT/local-play}"
if [[ -n "${FERRUMC_SERVER_BIN:-}" ]]; then
  SERVER_BIN="$FERRUMC_SERVER_BIN"
elif [[ -x "$ROOT/server/target/release/ferrumc" ]]; then
  # A copied binary's timestamp does not prove that its source is newer.
  # Prefer the ordinary host-local build; explicit paths still override it.
  SERVER_BIN="$ROOT/server/target/release/ferrumc"
else
  SERVER_BIN="$ROOT/local-natural/ferrumc"
fi
PORT="${FERRUMC_NATURAL_PORT:-25566}"
PRESET="${FERRUMC_NATURAL_PRESET:-$LATEST_PRESET}"
export RUST_LOG="${RUST_LOG:-debug}"
if [[ ! "$PORT" =~ ^[0-9]+$ ]] || (( ${#PORT} > 5 )) || (( 10#$PORT < 1 || 10#$PORT > 65535 )); then
  echo "FERRUMC_NATURAL_PORT must be a port from 1 to 65535." >&2
  exit 1
fi
case "$PRESET" in
  ferrumc_natural_preview|ferrumc_natural_grass_preview|ferrumc_natural_tree_preview|ferrumc_natural_survival_preview|ferrumc_natural_playable) ;;
  *) echo "Unsupported preview preset: $PRESET" >&2; exit 1 ;;
esac

if [[ ! -x "$SERVER_BIN" ]]; then
  echo "Missing executable: $SERVER_BIN" >&2
  echo "Build with cargo build --manifest-path server/Cargo.toml --release --locked -p ferrumc-app" >&2
  echo "Then run ./start.sh again; no generator or binary setting is needed." >&2
  exit 1
fi
SERVER_BIN="$(realpath "$SERVER_BIN")"
mkdir -p "$RUN_DIR"
RUN_DIR="$(cd "$RUN_DIR" && pwd)"
if [[ ! -f "$RUN_DIR/config.toml" ]]; then
  cat > "$RUN_DIR/config.toml" <<TOML
# Local Minecraft Java 1.21.8 Survival.
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
else
  echo "Existing config and saved world kept: $RUN_DIR" >&2
fi
# Preset names are simple identifiers. Only inspect the [world] table; the
# server remains the authority for TOML parsing and the stored world identity.
CONFIG_PRESET="$(awk '
  /^[[:space:]]*\[/ { world = ($0 ~ /^[[:space:]]*\[world\][[:space:]]*(#.*)?$/) }
  world && /^[[:space:]]*preset[[:space:]]*=/ {
    value = $0; sub(/^[^=]*=[[:space:]]*/, "", value)
    sub(/[[:space:]]*#.*/, "", value); gsub(/[[:space:]"\047]/, "", value)
    print value; exit
  }
' "$RUN_DIR/config.toml")"
if [[ -n "$CONFIG_PRESET" ]]; then
  echo "Generator: $CONFIG_PRESET"
  if [[ "$CONFIG_PRESET" != "$LATEST_PRESET" ]]; then
    echo "This save uses an older generator; its directory name does not add trees or structures." >&2
    echo "For the latest generator, use a fresh directory: FERRUMC_NATURAL_DIR=\"$ROOT/local-play-new\" ./start.sh" >&2
  fi
  if [[ -n "${FERRUMC_NATURAL_PRESET:-}" && "$CONFIG_PRESET" != "$PRESET" ]]; then
    echo "Requested preset $PRESET was not applied: an existing config is never rewritten." >&2
  fi
else
  echo "Generator: stored world identity (or server default for a fresh world; [world].preset is omitted)"
fi
echo "Minecraft Java 1.21.8: starting on 0.0.0.0:$PORT (connect using localhost or this host’s IP)"
echo "Executable: $SERVER_BIN"
echo "Logging: RUST_LOG=$RUST_LOG"
echo "Config: $RUN_DIR/config.toml; stop with Ctrl-C to save."
cd "$RUN_DIR"
exec "$SERVER_BIN" --config "$RUN_DIR/config.toml" --bind "0.0.0.0:$PORT" --bind-all-interfaces
