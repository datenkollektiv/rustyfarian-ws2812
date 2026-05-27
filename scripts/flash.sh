#!/usr/bin/env bash
set -euo pipefail
# flash.sh — auto-detect driver crate and flash a driver example
# Usage: scripts/flash.sh <example> [hal_dir [idf_dir]]   example: {driver}_{chip}_{name}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ $# -lt 1 ]; then
    printf 'Usage: %s <example>\n  example: {driver}_{chip}_{name}  e.g. hal_c6_pulse, idf_c3_rainbow\n' "$0" >&2
    exit 2
fi

example="$1"
hal_dir="${2:-target/hal}"
idf_dir="${3:-target/idf}"

prefix=$(printf '%s' "$example" | cut -d_ -f1)
case "$prefix" in
    hal) "$SCRIPT_DIR/run-example.sh" hal-ws2812 "$example" "$hal_dir" "$idf_dir" ;;
    idf) "$SCRIPT_DIR/run-example.sh" idf-ws2812 "$example" "$hal_dir" "$idf_dir" ;;
    *) printf 'Unknown driver prefix "%s". Expected hal or idf (e.g., hal_c6_rainbow)\n' "$prefix" >&2; exit 1 ;;
esac
