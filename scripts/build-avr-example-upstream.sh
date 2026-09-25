#!/usr/bin/env bash
set -euo pipefail
# build-avr-example-upstream.sh — build the AVR example against upstream avr-hal main
# Usage: scripts/build-avr-example-upstream.sh <REPO_ROOT> <AVR_NIGHTLY>
#
# The weekly early-warning check: drops the pinned `rev` so the example resolves
# avr-hal's tip instead. Pinning keeps PR builds reproducible but would hide upstream
# breakage, which this job exists to catch — a failure here means upstream moved, not
# that this repo is broken.
#
# Everything happens on a throwaway copy in a temp dir, so no tracked file is touched
# even transiently.

if [ $# -lt 2 ]; then
    printf 'Usage: %s <REPO_ROOT> <AVR_NIGHTLY>\n' "$0" >&2
    exit 2
fi

root="$1"
avr_nightly="$2"

probe="$(mktemp -d)"
trap 'rm -rf "$probe"' EXIT

mkdir -p "$probe/examples"
cp -R "$root/examples/avr-nano-rainbow" "$probe/examples/avr-nano-rainbow"
rm -rf "$probe/examples/avr-nano-rainbow/target"

# Both symlinks are required, not just `crates`. The example's path deps reach ../../crates,
# and those crates use `version.workspace = true`, which needs a workspace root to resolve.
# Cargo walks up the SYMLINK path rather than the resolved one, so linking only `crates`
# never reaches the real repo root and fails with `failed to find a workspace root`.
ln -s "$root/crates" "$probe/crates"
ln -s "$root/Cargo.toml" "$probe/Cargo.toml"

cd "$probe/examples/avr-nano-rainbow"

# Drop the pinned rev and the lockfile so Cargo resolves avr-hal main.
sed -i.bak '/^rev = /d' Cargo.toml && rm -f Cargo.toml.bak
rm -f Cargo.lock

# -Z build-std=core is passed on the command line rather than set in a child
# .cargo/config.toml: two config files MERGE their build-std arrays instead of
# overriding, which would yield ["std","panic_abort","core"] and fail on AVR.
cargo "+$avr_nightly" build --release -Z build-std=core --bins
