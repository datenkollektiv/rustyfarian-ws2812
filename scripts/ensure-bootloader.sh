#!/usr/bin/env bash
set -euo pipefail
# ensure-bootloader.sh — ensure the IDF-built bootloader is in the build cache
# Usage: scripts/ensure-bootloader.sh <chip> [hal_dir [idf_dir]]   chip: c3 | c6 | esp32 | s3
#
# espflash 4.x bundles its own bootloader, which rejected bare-metal esp-hal binaries
# (app descriptor format) and, while the pin was ESP-IDF v5.3.3, IDF binaries too
# (32 KB MMU page mismatch). The bootloader esp-idf-sys builds for the pinned ESP-IDF
# (.cargo/config.toml) boots both: v6.1 verified on ESP32-C3, C6 and S3 on 2026-11-12;
# not yet re-run on the classic ESP32.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./lib.sh
. "$SCRIPT_DIR/lib.sh"

if [ $# -lt 1 ]; then
    printf 'Usage: %s <chip>\n  chip: c3 | c6 | esp32 | s3\n' "$0" >&2
    exit 2
fi

chip="$1"
hal_dir="${2:-target/hal}"
idf_dir="${3:-target/idf}"

case "$chip" in
    c3)    idf_target="riscv32imc-esp-espidf"  ; idf_example="idf_c3_rainbow"    ; mcu="esp32c3"  ;;
    c6)    idf_target="riscv32imac-esp-espidf" ; idf_example="idf_c6_rainbow"    ; mcu="esp32c6"  ;;
    esp32) idf_target="xtensa-esp32-espidf"    ; idf_example="idf_esp32_rainbow" ; mcu="esp32"    ;;
    s3)    idf_target="xtensa-esp32s3-espidf"  ; idf_example="idf_s3_rainbow"    ; mcu="esp32s3"  ;;
    *) printf 'Unknown chip "%s". Supported: c3, c6, esp32, s3\n' "$chip" >&2; exit 1 ;;
esac

bl=$(find_idf_bootloader "$idf_target" "$idf_dir")
if [ -z "$bl" ]; then
    printf 'IDF bootloader not cached for %s -- building IDF example to populate it (requires cargo +esp)...\n' "$mcu" >&2
    "$SCRIPT_DIR/build-example.sh" "$idf_example" "$hal_dir" "$idf_dir"
fi
