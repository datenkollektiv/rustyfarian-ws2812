#!/usr/bin/env bash
set -euo pipefail
# setup.sh — install toolchain components for a fresh clone
# Usage: scripts/setup.sh <COMPONENT> <AVR_NIGHTLY>
#   COMPONENT: tools | hal | avr | esp | all
#
# Idempotent: every step is a no-op when already satisfied, so re-running after a
# pull that moved the AVR nightly is the supported way to catch up.
#
# `all` deliberately excludes `esp`: espup installs a full Xtensa toolchain and is
# opt-in. It prints the hint instead.

if [ $# -lt 2 ]; then
    printf 'Usage: %s <COMPONENT> <AVR_NIGHTLY>\n' "$0" >&2
    printf '  COMPONENT: tools | hal | avr | esp | all\n' >&2
    exit 2
fi

component="$1"
avr_nightly="$2"

setup_tools() {
    cargo install cargo-deny cargo-audit cargo-watch espup
}

setup_hal() {
    rustup target add riscv32imac-unknown-none-elf
    rustup target add riscv32imc-unknown-none-elf
}

setup_avr() {
    rustup toolchain install "$avr_nightly"
    rustup component add rust-src --toolchain "$avr_nightly"
    echo "AVR toolchain ready: $avr_nightly"
    echo "Ensure avr-gcc is installed: brew install avr-gcc (macOS) / apt install gcc-avr (Debian)"
}

setup_esp() {
    espup install
}

case "$component" in
    tools) setup_tools ;;
    hal) setup_hal ;;
    avr) setup_avr ;;
    esp) setup_esp ;;
    all)
        setup_tools
        setup_hal
        setup_avr
        echo ""
        echo "For Xtensa/ESP-IDF support, run: just setup esp"
        ;;
    *)
        echo "error: unknown component '$component'" >&2
        echo "usage: just setup [tools|hal|avr|esp|all]" >&2
        exit 1
        ;;
esac
