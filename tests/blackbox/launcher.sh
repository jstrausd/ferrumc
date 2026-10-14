#!/usr/bin/env bash
# Exercise launch selection/config preservation without opening any sockets.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CASE_DIR="$(mktemp -d)"
trap 'rm -rf "$CASE_DIR"' EXIT
mkdir -p "$CASE_DIR/scripts" "$CASE_DIR/server/target/release" "$CASE_DIR/local-natural"
cp "$ROOT/scripts/run-natural-preview.sh" "$CASE_DIR/scripts/"
LAUNCHER="$CASE_DIR/scripts/run-natural-preview.sh"
export MOCK_RECEIPT="$CASE_DIR/receipt"
unset FERRUMC_SERVER_BIN FERRUMC_NATURAL_DIR FERRUMC_NATURAL_PORT FERRUMC_NATURAL_PRESET RUST_LOG

fail() { echo "FAIL: $*" >&2; exit 1; }
contains() { [[ "$1" == *"$2"* ]] || fail "expected: $2"; }
absent() { [[ ! -e "$1" ]] || fail "unexpected path: $1"; }

help="$(bash "$LAUNCHER" --help)"
contains "$help" 'RUST_LOG'
absent "$CASE_DIR/local-play"
if bash "$LAUNCHER" >"$CASE_DIR/missing.log" 2>&1; then fail 'missing binary accepted'; fi
contains "$(cat "$CASE_DIR/missing.log")" 'cargo build'
absent "$CASE_DIR/local-play"

cat > "$CASE_DIR/server/target/release/ferrumc" <<'MOCK'
#!/usr/bin/env bash
printf '%s\n' "LOG=$RUST_LOG" "CWD=$PWD" "$@" > "$MOCK_RECEIPT"
MOCK
chmod +x "$CASE_DIR/server/target/release/ferrumc"
output="$(bash "$LAUNCHER" 2>&1)"
receipt="$(cat "$MOCK_RECEIPT")"
contains "$receipt" 'LOG=debug'
contains "$receipt" "CWD=$CASE_DIR/local-play"
contains "$receipt" '--bind-all-interfaces'
contains "$receipt" '0.0.0.0:25566'
contains "$output" 'Generator: ferrumc_natural_playable'
contains "$(cat "$CASE_DIR/local-play/config.toml")" 'preset = "ferrumc_natural_playable"'
contains "$output" "Executable: $CASE_DIR/server/target/release/ferrumc"

# Copy timestamps cannot establish source freshness. Prefer the ordinary build.
cp "$CASE_DIR/server/target/release/ferrumc" "$CASE_DIR/local-natural/ferrumc"
touch -t 202001010000 "$CASE_DIR/server/target/release/ferrumc"
output="$(bash "$LAUNCHER" 2>&1)"
contains "$output" "Executable: $CASE_DIR/server/target/release/ferrumc"

mkdir -p "$CASE_DIR/tree-preview"
cat > "$CASE_DIR/tree-preview/config.toml" <<'TOML'
view_distance = 3
[world] # a directory name never changes this
seed = 17
preset = 'ferrumc_natural_preview' # deliberately old
[access]
whitelist_enabled = false
TOML
cp "$CASE_DIR/tree-preview/config.toml" "$CASE_DIR/config-before"
output="$(RUST_LOG=warn FERRUMC_SERVER_BIN="$CASE_DIR/local-natural/ferrumc" \
  FERRUMC_NATURAL_DIR="$CASE_DIR/tree-preview" FERRUMC_NATURAL_PORT=25568 \
  FERRUMC_NATURAL_PRESET=ferrumc_natural_playable bash "$LAUNCHER" 2>&1)"
cmp "$CASE_DIR/config-before" "$CASE_DIR/tree-preview/config.toml" || fail 'old config changed'
receipt="$(cat "$MOCK_RECEIPT")"
contains "$receipt" 'LOG=warn'
contains "$receipt" '0.0.0.0:25568'
contains "$output" 'Generator: ferrumc_natural_preview'
contains "$output" 'older generator'
contains "$output" 'Requested preset ferrumc_natural_playable was not applied'
contains "$output" "Executable: $CASE_DIR/local-natural/ferrumc"

# Preset omission must not be presented as selecting the newest generator.
printf '[world]\nseed = 17\n' > "$CASE_DIR/tree-preview/config.toml"
output="$(FERRUMC_NATURAL_DIR="$CASE_DIR/tree-preview" bash "$LAUNCHER" 2>&1)"
contains "$output" 'stored world identity'
for bad_port in 0 65536 abc 999999999999999999; do
  if FERRUMC_NATURAL_PORT="$bad_port" bash "$LAUNCHER" >"$CASE_DIR/bad.log" 2>&1; then
    fail "invalid port accepted: $bad_port"
  fi
done
if bash "$LAUNCHER" --bind 0.0.0.0 >"$CASE_DIR/bad.log" 2>&1; then fail 'unexpected args accepted'; fi
contains "$(cat "$CASE_DIR/bad.log")" 'No arguments are needed'
echo 'Launcher checks passed (mock executable; no server or personal save opened).'
