#!/usr/bin/env bash
set -euo pipefail
# doctor.sh — check development prerequisites (RAM disk, sccache)
# Usage: scripts/doctor.sh <ramdisk> <hal_dir> <idf_dir> [idf_build_dir] [idf_build_glob]

if [ $# -lt 3 ]; then
    printf 'Usage: %s <ramdisk> <hal_dir> <idf_dir> [idf_build_dir] [idf_build_glob]\n' "$0" >&2
    exit 2
fi

ramdisk="$1"
hal_dir="$2"
idf_dir="$3"
idf_build_dir="${4:-}"
idf_build_glob="${5:-}"

if [ -d "$ramdisk" ]; then
    if [ -d "$ramdisk/targets/hal" ] && [ -d "$ramdisk/targets/idf" ]; then
        printf "  ramdisk    ok       %s\n" "$ramdisk"
        printf "  hal target ok       %s\n" "$hal_dir"
        printf "  idf target ok       %s\n" "$idf_dir"
    else
        printf "  ramdisk    PARTIAL  %s (subdirs missing — run: just ramdisk attach)\n" "$ramdisk"
        printf "  hal target fallback %s\n" "$hal_dir"
        printf "  idf target fallback %s\n" "$idf_dir"
    fi
else
    printf "  ramdisk    MISSING  run: just ramdisk attach\n"
    printf "  hal target fallback %s\n" "$hal_dir"
    printf "  idf target fallback %s\n" "$idf_dir"
fi

# Persistent IDF build-dir (v1.1 split): esp-idf-sys intermediates live here, off the RAM disk.
# Report materialization of THIS workspace's sharded dir, not just the shared cache root —
# the parent (~/Library/Caches/rustyfarian-cargo-build) exists as soon as any project has built,
# so checking it would falsely report "ok" before this workspace ever ran an IDF build.
if [ -n "$idf_build_dir" ]; then
    materialized=""
    if [ -n "$idf_build_glob" ]; then
        # $idf_build_glob carries Cargo's {workspace-path-hash}->*/* wildcard for the default cache.
        # A concrete RUSTYFARIAN_IDF_BUILD_DIR override has NO wildcard, and nullglob does nothing for a
        # literal path (it would "match" itself whether or not it exists) — so glob only when there is a
        # real wildcard, otherwise test the directory directly.
        case "$idf_build_glob" in
            *'*'*)
                shopt -s nullglob
                matches=( $idf_build_glob )
                shopt -u nullglob
                [ ${#matches[@]} -gt 0 ] && materialized=1
                ;;
            *)
                [ -d "$idf_build_glob" ] && materialized=1
                ;;
        esac
    fi
    if [ -n "$materialized" ]; then
        printf "  idf build  ok       %s\n" "$idf_build_dir"
    else
        printf "  idf build  configured %s (created on first IDF build)\n" "$idf_build_dir"
    fi
fi

if command -v sccache >/dev/null 2>&1; then
    if [ "${RUSTC_WRAPPER:-}" = "sccache" ]; then
        printf "  sccache    ok       %s\n" "$(sccache --version 2>/dev/null)"
    else
        printf "  sccache    --       installed but RUSTC_WRAPPER not set\n"
    fi
else
    printf "  sccache    MISSING  run: brew install sccache  (optional, speeds up cold builds)\n"
fi
