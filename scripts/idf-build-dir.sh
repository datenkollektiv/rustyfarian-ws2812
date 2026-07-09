#!/usr/bin/env bash
set -euo pipefail
# idf-build-dir.sh — resolve the persistent Cargo build-dir for IDF builds.
#
# Single source of truth for the v1.1 build-dir split
# (docs/features/separate-build-environments-v1.1.md): keep IDF *final* artifacts on the
# RAM disk (--target-dir) while relocating the bulky esp-idf-sys CMake tree (build-script
# OUT_DIR) to a persistent SSD cache via Cargo's build.build-dir.
#
# Usage:
#   idf-build-dir.sh            print the resolved build-dir (may contain {workspace-path-hash})
#   idf-build-dir.sh --config   print the --config flag that relocates Cargo's build-dir
#   idf-build-dir.sh --glob     print the build-dir with {workspace-path-hash} replaced by '*'
#
# NOTE: consume --config UNQUOTED via $(...) so the literal quotes survive to Cargo as TOML.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./lib.sh
. "$SCRIPT_DIR/lib.sh"

case "${1:-}" in
    "")       resolve_idf_build_dir ;;
    --config) idf_build_config_flag ;;
    --glob)   idf_build_dir_glob ;;
    *) printf 'Usage: %s [--config|--glob]\n' "$0" >&2; exit 2 ;;
esac
