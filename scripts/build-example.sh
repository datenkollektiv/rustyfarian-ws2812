#!/usr/bin/env bash
set -euo pipefail
# build-example.sh — build a driver example for a given chip
# Usage: scripts/build-example.sh <crate_alias> <example>
#   crate_alias: hal-ws2812 | idf-ws2812
#   example:     {driver}_{chip}_{name}  e.g. hal_c6_rainbow, idf_c3_rainbow

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_alias="$1"
example="$2"

prefix=$(printf '%s' "$example" | cut -d_ -f1)
chip=$(printf '%s' "$example" | cut -d_ -f2)

# Derive package from example prefix and validate the caller-provided crate_alias matches.
# Accepting crate_alias separately was redundant: prefix already identifies the crate unambiguously.
case "$prefix" in
    hal) pkg="rustyfarian-esp-hal-ws2812" ; expected_alias="hal-ws2812" ;;
    idf) pkg="rustyfarian-esp-idf-ws2812" ; expected_alias="idf-ws2812" ;;
    *) printf 'Unknown driver prefix "%s" in example "%s". Supported: hal, idf\n' "$prefix" "$example" >&2; exit 1 ;;
esac
if [ "$crate_alias" != "$expected_alias" ]; then
    printf 'Crate alias "%s" does not match example prefix "%s" (expected "%s")\n' "$crate_alias" "$prefix" "$expected_alias" >&2
    exit 1
fi

case "$prefix" in
    hal)
        case "$chip" in
            c3)    target="riscv32imc-unknown-none-elf"  ; chip_feature="esp32c3" ;;
            c6)    target="riscv32imac-unknown-none-elf" ; chip_feature="esp32c6" ;;
            esp32) target="xtensa-esp32-none-elf"        ; chip_feature="esp32"   ;;
            *) printf 'Unknown chip "%s" in example "%s". Supported: c3, c6, esp32\n' "$chip" "$example" >&2; exit 1 ;;
        esac
        printf 'Building %s for %s...\n' "$example" "$target"
        if [ "$chip" = "esp32" ]; then
            # Xtensa requires +esp toolchain and xtensa-esp-elf GCC.
            . "$SCRIPT_DIR/xtensa-toolchain.sh"
            setup_xtensa_toolchain
            cargo +esp build --release -Zbuild-std=core \
                --target "$target" \
                --no-default-features \
                --features "${chip_feature},unstable,led-effects,rt" \
                --example "$example" \
                -p "$pkg"
        else
            cargo build --release \
                --target "$target" \
                --no-default-features \
                --features "${chip_feature},unstable,led-effects,rt" \
                --example "$example" \
                -p "$pkg"
        fi
        ;;
    idf)
        case "$chip" in
            c3)    idf_target="riscv32imc-esp-espidf"  ; mcu="esp32c3"  ;;
            c6)    idf_target="riscv32imac-esp-espidf" ; mcu="esp32c6"  ;;
            esp32) idf_target="xtensa-esp32-espidf"    ; mcu="esp32"    ;;
            *) printf 'Unknown chip "%s" in IDF example "%s". Supported: c3, c6, esp32\n' "$chip" "$example" >&2; exit 1 ;;
        esac
        printf 'Building %s for %s...\n' "$example" "$idf_target"
        MCU="$mcu" cargo +esp build \
            --target "$idf_target" \
            --example "$example" \
            -p "$pkg"
        ;;
esac
