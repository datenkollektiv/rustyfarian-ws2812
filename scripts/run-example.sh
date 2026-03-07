#!/usr/bin/env bash
set -euo pipefail
# run-example.sh — build and flash a driver example
# Usage: scripts/run-example.sh <crate_alias> <example>
#   crate_alias: hal-ws2812 | idf-ws2812
#   example:     {driver}_{chip}_{name}  e.g. hal_c6_rainbow, idf_c3_rainbow

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ $# -lt 2 ]; then
    printf 'Usage: %s <crate_alias> <example>\n  crate_alias: hal-ws2812 | idf-ws2812\n  example:     {driver}_{chip}_{name}  e.g. hal_c6_pulse, idf_c3_rainbow\n' "$0" >&2
    exit 2
fi

crate_alias="$1"
example="$2"

prefix=$(printf '%s' "$example" | cut -d_ -f1)
chip=$(printf '%s' "$example" | cut -d_ -f2)

# Derive package from example prefix and validate the caller-provided crate_alias matches.
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
            c3)    hal_target="riscv32imc-unknown-none-elf"  ; idf_target="riscv32imc-esp-espidf"  ; mcu="esp32c3" ;;
            c6)    hal_target="riscv32imac-unknown-none-elf" ; idf_target="riscv32imac-esp-espidf" ; mcu="esp32c6" ;;
            esp32) hal_target="xtensa-esp32-none-elf"        ; idf_target="xtensa-esp32-espidf"    ; mcu="esp32"   ;;
            *) printf 'Unknown chip "%s" in example "%s". Supported: c3, c6, esp32\n' "$chip" "$example" >&2; exit 1 ;;
        esac
        # Base features required by all HAL examples.
        hal_features="${mcu},unstable,led-effects,rt"
        # Append optional features required by specific examples.
        name=$(printf '%s' "$example" | cut -d_ -f3-)
        case "$name" in
            smart_leds) hal_features="${hal_features},smart-leds" ;;
        esac
        printf 'Building %s for %s...\n' "$example" "$hal_target"
        "$SCRIPT_DIR/ensure-bootloader.sh" "$chip"
        if [ "$chip" = "esp32" ]; then
            # Xtensa requires +esp toolchain and xtensa-esp-elf GCC.
            # shellcheck source=./xtensa-toolchain.sh
            . "$SCRIPT_DIR/xtensa-toolchain.sh"
            setup_xtensa_toolchain
            cargo +esp build --release -Zbuild-std=core \
                --target "$hal_target" \
                --no-default-features \
                --features "$hal_features" \
                --example "$example" \
                -p "$pkg"
        else
            cargo build --release \
                --target "$hal_target" \
                --no-default-features \
                --features "$hal_features" \
                --example "$example" \
                -p "$pkg"
        fi
        bl=$(ls -t "$PWD/target/$idf_target/debug/build/esp-idf-sys-"*/out/build/bootloader/bootloader.bin 2>/dev/null | head -1 || true)
        if [ -z "$bl" ]; then
            printf 'Error: HAL examples require the IDF-built v5.3.3 bootloader; rebuild an IDF example to populate it: just build-example idf-ws2812 idf_%s_rainbow\n' "$chip" >&2
            exit 1
        fi
        printf 'Flashing %s with bootloader %s...\n' "$example" "$bl"
        espflash flash --bootloader "$bl" --ignore-app-descriptor "target/$hal_target/release/examples/$example"
        ;;
    idf)
        case "$chip" in
            c3)    idf_target="riscv32imc-esp-espidf"  ; mcu="esp32c3"  ;;
            c6)    idf_target="riscv32imac-esp-espidf" ; mcu="esp32c6"  ;;
            esp32) idf_target="xtensa-esp32-espidf"    ; mcu="esp32"    ;;
            *) printf 'Unknown chip "%s" in IDF example "%s". Supported: c3, c6, esp32\n' "$chip" "$example" >&2; exit 1 ;;
        esac
        # Base features required by all IDF examples.
        idf_features=""
        # Append optional features required by specific examples.
        name=$(printf '%s' "$example" | cut -d_ -f3-)
        case "$name" in
            smart_leds) idf_features="${idf_features:+${idf_features},}smart-leds" ;;
        esac
        printf 'Building %s for %s...\n' "$example" "$idf_target"
        if [ -n "$idf_features" ]; then
            MCU="$mcu" cargo +esp build \
                --target "$idf_target" \
                --features "$idf_features" \
                --example "$example" \
                -p "$pkg"
        else
            MCU="$mcu" cargo +esp build \
                --target "$idf_target" \
                --example "$example" \
                -p "$pkg"
        fi
        bl=$(ls -t "$PWD/target/$idf_target/debug/build/esp-idf-sys-"*/out/build/bootloader/bootloader.bin 2>/dev/null | head -1 || true)
        if [ -z "$bl" ]; then
            printf 'Warning: built bootloader not found, using espflash default (may fail on page-size mismatch)\n' >&2
            printf 'Flashing %s...\n' "$example"
            espflash flash --ignore-app-descriptor "target/$idf_target/debug/examples/$example"
        else
            printf 'Flashing %s with bootloader %s...\n' "$example" "$bl"
            espflash flash --bootloader "$bl" --ignore-app-descriptor "target/$idf_target/debug/examples/$example"
        fi
        ;;
esac
