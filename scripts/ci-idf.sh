#!/usr/bin/env bash
set -euo pipefail
# ci-idf.sh — build and link every ESP-IDF example of one chip (the cross-target-esp-idf CI job)
# Usage: scripts/ci-idf.sh <chip> [hal_dir [idf_dir]]
#   chip: c3 | c6 | esp32 | s3
#
# Globs the examples instead of listing them, so a new idf_<chip>_* example is linked in CI
# automatically. Zero matches is an error, so a typo or renamed chip cannot pass vacuously.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXAMPLES_DIR="$SCRIPT_DIR/../crates/rustyfarian-esp-idf-ws2812/examples"

if [ $# -lt 1 ]; then
    printf 'Usage: %s <chip> [hal_dir [idf_dir]]\n  chip: c3 | c6 | esp32 | s3\n' "$0" >&2
    exit 2
fi

chip="$1"
hal_dir="${2:-target/hal}"
idf_dir="${3:-target/idf}"

shopt -s nullglob
sources=("$EXAMPLES_DIR/idf_${chip}_"*.rs)
shopt -u nullglob

if [ ${#sources[@]} -eq 0 ]; then
    printf 'No ESP-IDF examples match idf_%s_*.rs in %s. Supported chips: c3, c6, esp32, s3\n' "$chip" "$EXAMPLES_DIR" >&2
    exit 1
fi

for src in "${sources[@]}"; do
    example="$(basename "$src" .rs)"
    printf '==> %s\n' "$example"
    "$SCRIPT_DIR/build-example.sh" "$example" "$hal_dir" "$idf_dir"
done

printf 'Linked %d ESP-IDF example(s) for %s\n' "${#sources[@]}" "$chip"
