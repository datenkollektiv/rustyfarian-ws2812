#!/usr/bin/env bash
set -euo pipefail
# avr.sh — build or flash a binary of the AVR Nano example crate (examples/avr-nano-rainbow)
# Usage: scripts/avr.sh <toolchain> <build|flash> [bin] [port]
#   bin:  a binary name (bitbang_demo, spi_rainbow, ...); empty = build every binary,
#         flash the default one (avr-nano-rainbow)
#   port: serial port for ravedude (RAVEDUDE_PORT); empty = $RAVEDUDE_PORT if set, else the one
#         port with the Nano's USB identity ($AVR_USB_ID, default 0403:6001, the FTDI FT232R
#         also declared as usb_id in bench/bench.toml). ravedude cannot guess an FTDI Nano itself.
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
usb_id="${AVR_USB_ID:-0403:6001}"
if [ "$action" = flash ] && [ -z "$port" ] && [ -z "${RAVEDUDE_PORT:-}" ]; then
    matches="$(uv run --quiet --no-project --with pyserial python -I -m serial.tools.list_ports -q "VID:PID=$usb_id")"
    count="$(printf '%s' "$matches" | grep -c . || true)"
    if [ "$count" -eq 1 ]; then
        port="$matches"
        printf 'Found the Nano on %s (USB %s)\n' "$port" "$usb_id" >&2
    elif [ "$count" -eq 0 ]; then
        printf 'No port with USB %s: plug in the Nano, or pass a port\n' "$usb_id" >&2
        exit 1
    else
        printf 'Several ports with USB %s:\n%s\nPass the port explicitly\n' "$usb_id" "$matches" >&2
        exit 1
    fi
fi
if [ -n "$port" ]; then export RAVEDUDE_PORT="$port"; fi

cd "$SCRIPT_DIR/../examples/avr-nano-rainbow"
exec cargo "${args[@]}"
