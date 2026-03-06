#!/usr/bin/env bash
set -euo pipefail
# ensure-bootloader.sh — ensure the IDF-built v5.3.3 bootloader is in the build cache
# Usage: scripts/ensure-bootloader.sh <chip>   chip: c3 | c6
#
# espflash 4.x bundles an ESP-IDF v5.5.1 bootloader that rejects both v5.3.3 IDF
# binaries (32 KB MMU page mismatch) and bare-metal esp-hal binaries (app descriptor
# format). The v5.3.3 bootloader built by esp-idf-sys works for both.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
chip="$1"

case "$chip" in
    c3)    idf_target="riscv32imc-esp-espidf"  ; idf_example="idf_c3_rainbow"    ;;
    c6)    idf_target="riscv32imac-esp-espidf" ; idf_example="idf_c6_rainbow"    ;;
    esp32) idf_target="xtensa-esp32-espidf"    ; idf_example="idf_esp32_rainbow" ;;
    *) printf 'Unknown chip "%s". Supported: c3, c6, esp32\n' "$chip" >&2; exit 1 ;;
esac

bl=$(ls -t "$PWD/target/$idf_target/debug/build/esp-idf-sys-"*/out/build/bootloader/bootloader.bin 2>/dev/null | head -1 || true)
if [ -z "$bl" ]; then
    printf 'IDF bootloader not cached for esp32%s -- building IDF example to populate it (requires cargo +esp)...\n' "$chip" >&2
    "$SCRIPT_DIR/build-example.sh" idf-ws2812 "$idf_example"
fi
