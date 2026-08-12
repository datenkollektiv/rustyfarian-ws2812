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

# idf_example_features <example>
# Prints the comma-separated extra cargo features an IDF example needs (or nothing).
# The feature is selected from the example's {name} part ({driver}_{chip}_{name}).
#
# IMPORTANT: this mapping MUST stay aligned with the `[[example]]`
# `required-features` declared in crates/rustyfarian-esp-idf-ws2812/Cargo.toml.
# When you add or rename an IDF example that needs a non-default feature, update
# both places together — otherwise the build silently drops the feature.
idf_example_features() {
    local example="$1"
    local name
    name=$(printf '%s' "$example" | cut -d_ -f3-)
    case "$name" in
        smart_leds) printf 'smart-leds' ;;
        rgb_cycle)  printf 'rgb-gpio' ;;
        rgb_pulse)  printf 'rgb-pwm' ;;
    esac
}

# resolve_idf_build_dir
# Prints the persistent Cargo build-dir used to relocate IDF build intermediates
# (esp-idf-sys OUT_DIR / CMake tree) off the RAM disk.
# Honours RUSTYFARIAN_IDF_BUILD_DIR; otherwise defaults under ~/Library/Caches using
# Cargo's {workspace-path-hash} template so each project gets an isolated subdir.
# See docs/features/separate-build-environments-v1.1.md.
resolve_idf_build_dir() {
    local dir
    # NB: do NOT fold this into "${VAR:-.../{workspace-path-hash}}" — the literal braces
    # in the default word confuse bash's ${:-} brace matching and leak a trailing '}'
    # into the path when the override IS set. Keep the if/else explicit.
    if [ -n "${RUSTYFARIAN_IDF_BUILD_DIR:-}" ]; then
        dir="$RUSTYFARIAN_IDF_BUILD_DIR"
    else
        dir="$HOME/Library/Caches/rustyfarian-cargo-build/{workspace-path-hash}"
    fi
    # The flag/glob are consumed UNQUOTED (to keep the TOML quotes / expand the shard
    # wildcard), so a path with whitespace would word-split into broken argv/globs.
    # Fail fast with a clear message instead of corrupting a cargo/find/rm invocation.
    case "$dir" in
        *[[:space:]]*)
            printf 'error: IDF build-dir must not contain whitespace: %s\n' "$dir" >&2
            printf '       set RUSTYFARIAN_IDF_BUILD_DIR to a space-free path.\n' >&2
            return 1
            ;;
    esac
    printf '%s' "$dir"
}

# idf_build_config_flag
# Prints the `--config` argument that moves Cargo's build-dir for IDF builds.
# IMPORTANT: consume this UNQUOTED via $(...). The literal double quotes must reach
# Cargo so it parses the value as a TOML string; wrapping the whole thing in shell
# quotes would strip them and Cargo would reject the bare path as invalid TOML.
idf_build_config_flag() {
    local dir
    dir="$(resolve_idf_build_dir)" || return 1
    printf -- '--config=build.build-dir="%s"' "$dir"
}

# idf_build_dir_glob
# Prints the build-dir with Cargo's {workspace-path-hash} template replaced by a shell
# glob. Cargo expands the template itself, so the shell cannot know the concrete subdir;
# globbing across hash dirs is how discovery/clean locate the relocated tree.
#
# NOTE: Cargo expands {workspace-path-hash} to a *sharded two-level* path
# (e.g. "10/fa8c3bedaaa338"), so the wildcard is "*/*", not a single "*".
# Verified empirically on cargo 1.95.0-nightly (2026-03). If a concrete
# RUSTYFARIAN_IDF_BUILD_DIR override is used, no template is present and the path is
# returned unchanged.
idf_build_dir_glob() {
    local raw
    raw="$(resolve_idf_build_dir)" || return 1
    printf '%s' "${raw//\{workspace-path-hash\}/*/*}"
}

# find_idf_bootloader <idf_target> [idf_dir]
# Prints the path of the single IDF-built bootloader to stdout.
# Prints nothing if no bootloader is found.
# Exits with an error if multiple candidates are found (ambiguous — build dirs must be cleaned first).
# Multi-match grows more likely over time in a shared persistent cache (stale esp-idf-sys-<hash>
# dirs from old manifests/sdkconfig accumulate); the error tells the operator to run
# `just clean-idf` (or `just clean-idf-cache`) rather than have discovery silently guess.
#
# The bootloader lives inside esp-idf-sys's OUT_DIR. With build.build-dir set (the v1.1
# split), OUT_DIR is relocated to the persistent build-dir, so that location is searched
# first; the legacy in-target-dir location is kept as a fallback for pre-split builds.
find_idf_bootloader() {
    local idf_target="$1"
    local idf_dir="${2:-target/idf}"
    # Resolve idf_dir to an absolute path; $PWD prefix only for relative paths.
    local base_dir
    case "$idf_dir" in
        /*) base_dir="$idf_dir" ;;
        *)  base_dir="$PWD/$idf_dir" ;;
    esac
    local build_glob
    build_glob="$(idf_build_dir_glob)"
    # nullglob makes each pattern vanish (not stay a literal string) when nothing matches,
    # so the zero/one/many logic below is reliable without an additional -e check.
    # $build_glob is left UNQUOTED so its {workspace-path-hash}->* wildcard expands.
    shopt -s nullglob
    local bl_candidates=(
        ${build_glob}/${idf_target}/debug/build/esp-idf-sys-*/out/build/bootloader/bootloader.bin
        "$base_dir/$idf_target/debug/build"/esp-idf-sys-*/out/build/bootloader/bootloader.bin
    )
    shopt -u nullglob
    if [ ${#bl_candidates[@]} -gt 0 ]; then
        if [ ${#bl_candidates[@]} -gt 1 ]; then
            printf 'Error: multiple IDF-built bootloaders found for target "%s".\n' "$idf_target" >&2
            printf 'A Cargo.toml change (new dependency version or [[example]]) gives esp-idf-sys a\n' >&2
            printf 'fresh build hash, leaving the previous esp-idf-sys-* directory behind.\n\n' >&2
            printf 'Note: `cargo clean -p esp-idf-sys` does NOT help while the build-dir split is\n' >&2
            printf 'active — the artifacts live in the relocated cache below, not under target/.\n\n' >&2
            printf 'Fix: just clean-idf-stale    (drops superseded dirs, keeps the newest per target)\n' >&2
            printf 'Fix (reset all): just clean-idf-cache    (full rebuild, incl. other architectures)\n\n' >&2
            printf 'Note: a dependency bump rehashes esp-idf-sys for EVERY IDF target, but each one\n' >&2
            printf 'only grows a second directory when next built — so this recurs per architecture.\n' >&2
            printf '`just clean-idf-stale` sweeps them all at once.\n\n' >&2
            printf 'Candidates (listed in glob order; compare the timestamps):\n' >&2
            for cand in "${bl_candidates[@]}"; do
                # Display mtime so the stale directory is obvious; BSD stat first, GNU as fallback.
                cand_mtime="$(stat -f '%Sm' -t '%Y-%m-%d %H:%M' "$cand" 2>/dev/null \
                    || stat -c '%y' "$cand" 2>/dev/null | cut -c1-16 \
                    || echo 'unknown')"
                printf '  [%s]  %s\n' "$cand_mtime" "$cand" >&2
            done
            exit 1
        fi
        echo "${bl_candidates[0]}"
    fi
}
