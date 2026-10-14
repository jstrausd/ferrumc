#!/usr/bin/env bash
# Normal local Survival startup; optional settings are handled by the launcher.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$ROOT/scripts/run-natural-preview.sh" "$@"
