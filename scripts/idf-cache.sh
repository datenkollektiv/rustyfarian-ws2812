#!/usr/bin/env bash
set -euo pipefail
# idf-cache.sh — manage the relocated ESP-IDF build cache (esp-idf-sys CMake trees)
# Usage: scripts/idf-cache.sh <command> [ARGS] [--dry-run]
#   clean <IDF_DIR>  remove esp-idf-sys hash dirs for every IDF target, in both the
#                    persistent cache and the legacy in-target-dir location
#   clean-stale      drop superseded esp-idf-sys dirs, keeping the newest per target
#   clean-all        remove the persistent build-dir cache entirely
#   info             print the resolved build-dir plumbing and materialisation state
# --dry-run prints what would be removed without deleting anything (clean* only).

# Every glob in this script wants nullglob semantics: a non-matching pattern must vanish
# rather than reach `rm` as a literal string containing `*`. Enabled once here so no
# function has to set/restore it locally.
shopt -s nullglob

# Anchor helper lookups to this script's directory rather than the caller's cwd —
# a recipe (or a nested shell) may have chdir'd elsewhere before invoking us.
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Target policy — deliberately discovery-based, not a curated list.
#
# A dependency bump rehashes esp-idf-sys for EVERY IDF target at once, so cleanup must
# sweep all of them. This workspace currently builds riscv32imac-esp-espidf (C6),
# riscv32imc-esp-espidf (C3) and xtensa-esp32-espidf (WROOM-32), but hardcoding that list
# means a target added later is silently skipped by `clean` while `clean-stale` still
# finds it — a mismatch that would surface as a "multiple IDF-built bootloaders" error
# no amount of cleaning could fix. Both commands therefore match `*/debug/build` under
# the cache root, which is specific enough that only IDF target dirs qualify.

# Set by resolve_matches(); declared here so the global is visible rather than implied.
matches=()
dry_run=false

build_dir_glob() { "$here/idf-build-dir.sh" --glob; }

# resolve_matches PATTERN — expand a build-dir pattern into concrete paths.
# OUTPUT: populates the global array `matches` (callers read it directly).
#
# Glob only when the pattern carries the {workspace-path-hash}->*/* wildcard (the default
# cache layout). A concrete RUSTYFARIAN_IDF_BUILD_DIR override is a literal path that
# nullglob cannot distinguish from a real match, so probe it directly instead.
resolve_matches() {
    local pattern="$1"
    case "$pattern" in
        *'*'*)
            # IFS= suppresses word splitting while leaving pathname expansion enabled,
            # so a cache path containing spaces survives intact.
            local IFS=
            matches=($pattern)
            ;;
        *)
            matches=()
            [ -d "$pattern" ] && matches=("$pattern")
            ;;
    esac
}

# remove_all DESCRIPTION PATH... — delete the given paths, or list them under --dry-run.
remove_all() {
    local what="$1"; shift
    if [ "$#" -eq 0 ]; then
        printf 'No %s to remove.\n' "$what"
        return
    fi
    if [ "$dry_run" = true ]; then
        printf 'Would remove %d %s:\n' "$#" "$what"
    else
        rm -rf "$@"
        printf 'Removed %d %s:\n' "$#" "$what"
    fi
    local p
    for p in "$@"; do printf '  %s\n' "${p%/}"; done
}

cmd_clean() {
    local idf_dir="${1:?usage: idf-cache.sh clean <IDF_DIR>}"
    local glob_base victims=()
    glob_base="$(build_dir_glob)"
    # The esp-idf-sys build tree is relocated to the persistent build-dir (v1.1 split), so
    # remove it there; also sweep the legacy in-target-dir location for pre-split builds.
    #
    # Collect first, delete second. $glob_base carries a wildcard so it must stay unquoted;
    # $idf_dir is a concrete path and is quoted. IFS= suppresses word splitting in both
    # cases while leaving pathname expansion on — without it a path containing a space is
    # split, and the surviving fragment is a truncated *prefix* that rm would delete
    # instead of the intended directory.
    local IFS=
    victims+=($glob_base/*/debug/build/esp-idf-sys-*/)
    victims+=("$idf_dir"/*/debug/build/esp-idf-sys-*/)
    unset IFS
    remove_all "esp-idf-sys build dir(s)" "${victims[@]}"
}

cmd_clean_stale() {
    # A dependency bump rehashes esp-idf-sys for EVERY IDF target, but each target only
    # grows its second directory the next time it is built — so the "multiple IDF-built
    # bootloaders" error surfaces one architecture at a time. This sweeps them all at once,
    # keeping the newest dir per target so the current build is preserved.
    local glob_base removed=0 build_dir target newest d
    glob_base="$(build_dir_glob)"
    local IFS=
    local build_dirs=($glob_base/*/debug/build)
    unset IFS
    for build_dir in "${build_dirs[@]}"; do
        local dirs=("$build_dir"/esp-idf-sys-*/)
        [ ${#dirs[@]} -le 1 ] && continue
        target="$(basename "$(dirname "$(dirname "$build_dir")")")"
        # Newest first, so everything after index 0 is superseded. `ls -dt` is safe here:
        # these directory names are Cargo-generated `esp-idf-sys-<hex>` with no spaces or
        # newlines, and one path per output line survives an enclosing path with spaces.
        newest="$(ls -dt "${dirs[@]}" | head -1)"
        for d in "${dirs[@]}"; do
            [ "${d%/}" = "${newest%/}" ] && continue
            if [ "$dry_run" = true ]; then
                printf 'would remove %-7s %s (%s)\n' "$(du -sh "$d" | cut -f1)" "$(basename "${d%/}")" "$target"
            else
                printf 'removing %-7s %s (%s)\n' "$(du -sh "$d" | cut -f1)" "$(basename "${d%/}")" "$target"
                rm -rf "$d"
            fi
            removed=$((removed + 1))
        done
        printf 'kept              %s (%s)\n' "$(basename "${newest%/}")" "$target"
    done
    if [ "$removed" -eq 0 ]; then
        echo "No superseded esp-idf-sys build dirs found — nothing to do."
    elif [ "$dry_run" = true ]; then
        echo "Would remove $removed superseded esp-idf-sys build dir(s)."
    else
        echo "Removed $removed superseded esp-idf-sys build dir(s)."
    fi
}

cmd_clean_all() {
    local glob_base
    glob_base="$(build_dir_glob)"
    resolve_matches "$glob_base"
    if [ ${#matches[@]} -eq 0 ]; then
        echo "No IDF build cache to remove ($glob_base)"
        return
    fi
    remove_all "IDF build cache path(s)" "${matches[@]}"
}

cmd_info() {
    printf 'resolved : %s\n' "$("$here/idf-build-dir.sh")"
    printf 'config   : %s\n' "$("$here/idf-build-dir.sh" --config)"
    printf 'glob     : %s\n' "$(build_dir_glob)"
    resolve_matches "$(build_dir_glob)"
    if [ ${#matches[@]} -eq 0 ]; then
        printf 'state    : not yet materialized (created on first IDF build)\n'
    else
        printf 'state    : materialized\n'
        local m
        for m in "${matches[@]}"; do printf '           %s\n' "$m"; done
    fi
}

usage() {
    sed -n '3,10p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

# Pull --dry-run out of the argument list wherever it appears, so it works both as
# `clean-stale --dry-run` and `clean --dry-run <IDF_DIR>`.
args=()
for a in "$@"; do
    if [ "$a" = "--dry-run" ]; then dry_run=true; else args+=("$a"); fi
done
set -- "${args[@]}"

case "${1:-}" in
    clean)       shift; cmd_clean "$@" ;;
    clean-stale) cmd_clean_stale ;;
    clean-all)   cmd_clean_all ;;
    info)        cmd_info ;;
    -h|--help)   usage ;;
    *)
        printf 'Error: unknown or missing command: %s\n\n' "${1:-<none>}" >&2
        usage >&2
        exit 1
        ;;
esac
