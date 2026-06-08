#!/usr/bin/env bash
# lib.sh — shared helper functions for scripts/
# Source this file; do not execute it directly.

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    printf 'Error: lib.sh must be sourced, not executed directly.\n' >&2
    exit 2
fi

# is_ramdisk_mounted <path>
# Returns 0 if <path> is a live mounted volume on macOS, 1 otherwise.
# Uses `diskutil info` rather than parsing `mount` output, so a stale
# /Volumes/<name> directory (left over from a failed detach) is correctly
# reported as unmounted.
is_ramdisk_mounted() {
    local path="$1"
    [ -n "$path" ] || return 1
    diskutil info "$path" >/dev/null 2>&1
}

# find_idf_bootloader <idf_target> [idf_dir]
# Prints the path of the single IDF-built bootloader to stdout.
# Prints nothing if no bootloader is found.
# Exits with an error if multiple candidates are found (ambiguous — build dirs must be cleaned first).
find_idf_bootloader() {
    local idf_target="$1"
    local idf_dir="${2:-target/idf}"
    # Resolve idf_dir to an absolute path; $PWD prefix only for relative paths.
    local base_dir
    case "$idf_dir" in
        /*) base_dir="$idf_dir" ;;
        *)  base_dir="$PWD/$idf_dir" ;;
    esac
    # nullglob makes the array empty (not a literal pattern string) when nothing matches,
    # so the zero/one/many logic below is reliable without an additional -e check.
    shopt -s nullglob
    local bl_candidates=( "$base_dir/$idf_target/debug/build"/esp-idf-sys-*/out/build/bootloader/bootloader.bin )
    shopt -u nullglob
    if [ ${#bl_candidates[@]} -gt 0 ]; then
        if [ ${#bl_candidates[@]} -gt 1 ]; then
            printf 'Error: multiple IDF-built bootloaders found for target "%s".\n' "$idf_target" >&2
            printf 'Run: cargo clean -p esp-idf-sys, or remove unused esp-idf-sys-* build directories.\nCandidates:\n' >&2
            for cand in "${bl_candidates[@]}"; do
                printf '  %s\n' "$cand" >&2
            done
            exit 1
        fi
        echo "${bl_candidates[0]}"
    fi
}
