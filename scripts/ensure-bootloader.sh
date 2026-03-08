#!/usr/bin/env bash
set -euo pipefail
# ensure-bootloader.sh — ensure the IDF-built v5.3.3 bootloader is in the build cache
# Usage: scripts/ensure-bootloader.sh <chip>   chip: c3 | c6
#
# espflash 4.x bundles an ESP-IDF v5.5.1 bootloader that rejects both v5.3.3 IDF
# binaries (32 KB MMU page mismatch) and bare-metal esp-hal binaries (app descriptor
# format). The v5.3.3 bootloader built by esp-idf-sys works for both.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./lib.sh
. "$SCRIPT_DIR/lib.sh"

if [ $# -lt 1 ]; then
    printf 'Usage: %s <chip>\n  chip: c3 | c6 | esp32\n' "$0" >&2
    exit 2
fi

chip="$1"

case "$chip" in
    c3)    idf_target="riscv32imc-esp-espidf"  ; idf_example="idf_c3_rainbow"    ; mcu="esp32c3"  ;;
    c6)    idf_target="riscv32imac-esp-espidf" ; idf_example="idf_c6_rainbow"    ; mcu="esp32c6"  ;;
    esp32) idf_target="xtensa-esp32-espidf"    ; idf_example="idf_esp32_rainbow" ; mcu="esp32"    ;;
    *) printf 'Unknown chip "%s". Supported: c3, c6, esp32\n' "$chip" >&2; exit 1 ;;
esac

bl=$(find_idf_bootloader "$idf_target")
if [ -z "$bl" ]; then
    printf 'IDF bootloader not cached for %s -- building IDF example to populate it (requires cargo +esp)...\n' "$mcu" >&2
    "$SCRIPT_DIR/build-example.sh" idf-ws2812 "$idf_example"
fi
