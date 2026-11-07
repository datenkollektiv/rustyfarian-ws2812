#!/usr/bin/env bash
set -euo pipefail
# avr.sh — build or flash a binary of the AVR Nano example crate (examples/avr-nano-rainbow)
# Usage: scripts/avr.sh <toolchain> <build|flash> [bin] [port]
#   bin:  a binary name (bitbang_demo, spi_rainbow, ...); empty = build every binary,
#         flash the default one (avr-nano-rainbow)
#   port: serial port for ravedude (RAVEDUDE_PORT); empty = ravedude finds the board itself
#
# build-std stays on the command line: the workspace .cargo/config.toml sets build-std for
# ESP-IDF, and Cargo would merge it with ours (see examples/avr-nano-rainbow/.cargo/config.toml).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ $# -lt 2 ]; then
    printf 'Usage: %s <toolchain> <build|flash> [bin] [port]\n' "$0" >&2
    exit 2
fi
toolchain="$1"
action="$2"
bin="${3:-}"
port="${4:-}"

case "$action" in
    build) cmd=build ;;
    flash) cmd=run ;;
    *) printf 'Unknown action "%s". Expected build or flash\n' "$action" >&2; exit 2 ;;
esac

args=(+"$toolchain" "$cmd" --release -Z build-std=core)
if [ -n "$bin" ]; then args+=(--bin "$bin"); fi
if [ -n "$port" ]; then export RAVEDUDE_PORT="$port"; fi

cd "$SCRIPT_DIR/../examples/avr-nano-rainbow"
exec cargo "${args[@]}"
