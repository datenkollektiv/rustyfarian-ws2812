#!/usr/bin/env bash
set -euo pipefail
# build-example.sh — build a driver example for a given chip
# Usage: scripts/build-example.sh <crate_alias> <example> [hal_dir [idf_dir]]
#   crate_alias: hal-ws2812 | idf-ws2812
#   example:     {driver}_{chip}_{name}  e.g. hal_c6_rainbow, idf_c3_rainbow

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ $# -lt 2 ]; then
    printf 'Usage: %s <crate_alias> <example>\n  crate_alias: hal-ws2812 | idf-ws2812\n  example:     {driver}_{chip}_{name}  e.g. hal_c6_pulse, idf_c3_rainbow\n' "$0" >&2
    exit 2
fi

crate_alias="$1"
example="$2"
hal_dir="${3:-target/hal}"
idf_dir="${4:-target/idf}"

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
            c3)    target="riscv32imc-unknown-none-elf"  ; mcu="esp32c3" ;;
            c6)    target="riscv32imac-unknown-none-elf" ; mcu="esp32c6" ;;
            esp32) target="xtensa-esp32-none-elf"        ; mcu="esp32"   ;;
            *) printf 'Unknown chip "%s" in example "%s". Supported: c3, c6, esp32\n' "$chip" "$example" >&2; exit 1 ;;
        esac
        # Base features required by all HAL examples.
        #
        # esp-println is enabled per-chip, based on what current examples actually use:
        #   - C6:    every example today calls esp_println::println from its panic handler.
        #            If a future C6 example is silent, this rule will over-enable for it —
        #            convert to a per-example case below or drop esp-println for that example.
        #   - C3:    examples use a silent panic handler; esp-println is not needed.
        #   - esp32: the esp-println dep is pinned to features = ["jtag-serial"] in Cargo.toml.
        #            That transport requires USB-Serial-JTAG hardware which is RISC-V only
        #            (C3/C6/H2/P4/S3); Xtensa LX6 lacks the peripheral. Switching to a
        #            different transport (uart, rtt) is possible but not currently wired up.
        case "$chip" in
            c6) hal_features="${mcu},unstable,pennant,rt,esp-println" ;;
            *)  hal_features="${mcu},unstable,pennant,rt" ;;
        esac
        # Append optional features required by specific examples.
        name=$(printf '%s' "$example" | cut -d_ -f3-)
        case "$name" in
            smart_leds) hal_features="${hal_features},smart-leds" ;;
            *_async)    hal_features="${hal_features},async" ;;
        esac
        printf 'Building %s for %s...\n' "$example" "$target"
        if [ "$chip" = "esp32" ]; then
            # Xtensa requires +esp toolchain and xtensa-esp-elf GCC.
            # shellcheck source=./xtensa-toolchain.sh
            . "$SCRIPT_DIR/xtensa-toolchain.sh"
            setup_xtensa_toolchain
            cargo +esp build --release -Zbuild-std=core \
                --target "$target" \
                --target-dir "$hal_dir" \
                --no-default-features \
                --features "$hal_features" \
                --example "$example" \
                -p "$pkg"
        else
            cargo build --release \
                --target "$target" \
                --target-dir "$hal_dir" \
                --no-default-features \
                --features "$hal_features" \
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
                --target-dir "$idf_dir" \
                --features "$idf_features" \
                --example "$example" \
                -p "$pkg"
        else
            MCU="$mcu" cargo +esp build \
                --target "$idf_target" \
                --target-dir "$idf_dir" \
                --example "$example" \
                -p "$pkg"
        fi
        ;;
esac
