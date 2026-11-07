#!/usr/bin/env bash
set -euo pipefail
# semver.sh — compare each crate's public API with a published release (cargo-semver-checks)
# Usage: scripts/semver.sh <baseline> <host_target> <hal_target>   baseline: "" = latest on crates.io
#
# Each crate is checked on the target and features its check recipes use.
# rustyfarian-esp-idf-ws2812 is not covered: its docs need a full ESP-IDF build, twice.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

if [ $# -lt 3 ]; then
    printf 'Usage: %s <baseline> <host_target> <hal_target>\n' "$0" >&2
    exit 2
fi
baseline="$1"
host_target="$2"
hal_target="$3"

base=()
if [ -n "$baseline" ]; then base=(--baseline-version "$baseline"); fi

for p in bunting ferriswheel pennant; do
    cargo semver-checks -p "$p" --target "$host_target" "${base[@]}"
done
cargo semver-checks -p rustyfarian-avr-ws2812 --target "$host_target" \
    --only-explicit-features --features bitbang,smart-leds-trait,pennant "${base[@]}"

# The esp-hal driver runs from /tmp with a rustc wrapper; two things break its rustdoc build:
# 1. the target probe fails under cargo-semver-checks' --cap-lints=allow (see rustc-probe-wrapper.sh);
# 2. .cargo/config.toml, found from the working directory, sets build-std = ["std", ...], which
#    cannot build for a bare-metal target. Docs link nothing, so they need neither it nor the
#    -Tlinkall.x linker script.
(cd /tmp && RUSTC_WRAPPER="$SCRIPT_DIR/rustc-probe-wrapper.sh" cargo semver-checks \
    --manifest-path "$REPO_DIR/crates/rustyfarian-esp-hal-ws2812/Cargo.toml" \
    --target "$hal_target" --only-explicit-features --features esp32c6,unstable,pennant,async,smart-leds \
    "${base[@]}")
