#!/usr/bin/env bash
set -euo pipefail
# rustc-probe-wrapper.sh — RUSTC_WRAPPER that keeps rustc's warnings on cargo's target probe
# Usage: RUSTC_WRAPPER=scripts/rustc-probe-wrapper.sh cargo ...   (cargo passes: <rustc> <args...>)
#
# cargo-semver-checks injects `--cap-lints=allow`, which also silences rustc's "dropping
# unsupported crate type" warning. Cargo parses that warning to learn that a bare-metal target
# cannot build dylib/cdylib; without it the probe fails with "output of --print=file-names
# missing". Only the probe (the call with --print=file-names) gets the flag removed; real
# builds keep their capped lints.

rustc="$1"
shift
probe=0
for a in "$@"; do
    if [ "$a" = "--print=file-names" ]; then probe=1; fi
done
args=()
for a in "$@"; do
    if [ "$probe" = 1 ] && [ "$a" = "--cap-lints=allow" ]; then continue; fi
    args+=("$a")
done
exec "$rustc" "${args[@]}"
