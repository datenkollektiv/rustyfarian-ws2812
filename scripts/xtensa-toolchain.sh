#!/usr/bin/env bash
# xtensa-toolchain.sh — shared helper: add xtensa-esp32-elf-gcc to PATH if needed
# Source this file; do not execute it directly.

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    printf 'Error: xtensa-toolchain.sh must be sourced, not executed directly.\n' >&2
    exit 2
fi

setup_xtensa_toolchain() {
    if ! command -v xtensa-esp32-elf-gcc >/dev/null 2>&1; then
        local xtensa_bin=""
        local rustup_candidates=("$HOME/.rustup/toolchains/esp/xtensa-esp-elf/"*/xtensa-esp-elf/bin)
        if [ ${#rustup_candidates[@]} -gt 0 ] && [ -d "${rustup_candidates[0]}" ]; then
            if [ ${#rustup_candidates[@]} -gt 1 ]; then
                printf 'Error: multiple xtensa toolchain directories found under:\n' >&2
                printf '  ~/.rustup/toolchains/esp/xtensa-esp-elf/*/xtensa-esp-elf/bin\n' >&2
                printf 'Remove or rename extra versions so only one remains.\n' >&2
                return 1
            fi
            xtensa_bin="${rustup_candidates[0]}"
        fi
        if [ -z "$xtensa_bin" ]; then
            local espressif_candidates=("$HOME/.espressif/tools/xtensa-esp-elf/"*/xtensa-esp-elf/bin)
            if [ ${#espressif_candidates[@]} -gt 0 ] && [ -d "${espressif_candidates[0]}" ]; then
                if [ ${#espressif_candidates[@]} -gt 1 ]; then
                    printf 'Error: multiple xtensa toolchain directories found under:\n' >&2
                    printf '  ~/.espressif/tools/xtensa-esp-elf/*/xtensa-esp-elf/bin\n' >&2
                    printf 'Remove or rename extra versions so only one remains.\n' >&2
                    return 1
                fi
                xtensa_bin="${espressif_candidates[0]}"
            fi
        fi
        if [ -n "$xtensa_bin" ]; then
            export PATH="$xtensa_bin:$PATH"
        else
            printf 'Error: xtensa-esp32-elf-gcc not found. Searched:\n' >&2
            printf '  ~/.rustup/toolchains/esp/xtensa-esp-elf/*/xtensa-esp-elf/bin\n' >&2
            printf '  ~/.espressif/tools/xtensa-esp-elf/*/xtensa-esp-elf/bin\n' >&2
            printf 'Install ESP-IDF toolchain or source your toolchain environment export script.\n' >&2
            return 1
        fi
    fi
}
