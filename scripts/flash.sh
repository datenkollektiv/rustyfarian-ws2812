#!/usr/bin/env bash
set -euo pipefail
# flash.sh — auto-detect driver crate and flash a driver example
# Usage: scripts/flash.sh <example>   example: {driver}_{chip}_{name}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
example="$1"

prefix=$(printf '%s' "$example" | cut -d_ -f1)
case "$prefix" in
    hal) "$SCRIPT_DIR/run-example.sh" hal-ws2812 "$example" ;;
    idf) "$SCRIPT_DIR/run-example.sh" idf-ws2812 "$example" ;;
    *) printf 'Unknown driver prefix "%s". Expected hal or idf (e.g., hal_c6_rainbow)\n' "$prefix" >&2; exit 1 ;;
esac
