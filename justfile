# Rustyfarian WS2812 — development tasks
#
# The workspace defaults to the ESP32 target (riscv32imac-esp-espidf) via
# .cargo/config.toml, so every recipe that touches platform-independent crates
# explicitly passes --target to override it.

host_target := `rustc -vV | sed -n 's/^host: //p'`
pure_crates := "-p ws2812-pure -p ferriswheel -p led-effects"

# list available recipes (default)
_default:
    @just --list

# build platform-independent crates
build:
    cargo build {{ pure_crates }} --target {{ host_target }}

# build all crates including ESP-IDF (requires espup)
build-all:
    cargo build

# check platform-independent crates (no ESP toolchain required)
check:
    cargo check {{ pure_crates }} --target {{ host_target }}

# check all crates including ESP-IDF (requires espup)
check-all:
    cargo check

# run clippy on platform-independent crates
clippy:
    cargo clippy {{ pure_crates }} --target {{ host_target }} -- -D warnings

# run clippy on all crates including ESP-IDF (requires espup)
clippy-all:
    cargo clippy -- -D warnings

# run unit and doc tests
test:
    cargo test {{ pure_crates }} --target {{ host_target }}

# run tests with stdout/stderr visible
test-verbose:
    cargo test {{ pure_crates }} --target {{ host_target }} -- --nocapture

# test a specific crate (e.g., just test-crate ferriswheel)
test-crate crate:
    cargo test -p {{ crate }} --target {{ host_target }}

# format all code
fmt:
    cargo fmt

# check formatting without modifying files
fmt-check:
    cargo fmt -- --check

# build rustdoc for platform-independent crates
doc:
    cargo doc {{ pure_crates }} --target {{ host_target }} --no-deps

# build and open docs in browser
doc-open:
    cargo doc {{ pure_crates }} --target {{ host_target }} --no-deps --open

# check dependency licenses, advisories, and bans
deny:
    cargo deny check

# update dependencies
update:
    cargo update

# clean build artifacts
clean:
    cargo clean

# watch and re-run tests on file changes (requires cargo-watch)
watch:
    cargo watch -x 'test {{ pure_crates }} --target {{ host_target }}'

# full pre-commit verification: format, check, lint, test
verify: fmt check clippy test

# CI-equivalent verification (non-modifying): format check, deny, check, lint, test
ci: fmt-check deny check clippy test
