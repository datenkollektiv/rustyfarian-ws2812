# Rustyfarian WS2812 — development tasks
#
# The workspace defaults to the ESP32 target (riscv32imac-esp-espidf) via
# .cargo/config.toml, so every recipe that touches platform-independent crates
# explicitly passes --target to override it.

host_target := `scripts/host-target.sh`
pure_crates := "-p ws2812-pure -p ferriswheel -p led-effects"
hal_target := "riscv32imac-unknown-none-elf"
hal_crate := "-p rustyfarian-esp-hal-ws2812"

# list available recipes (default)
_default:
    @just --list

# --- Build & Check --------------------------------------------------------

# build platform-independent crates
build:
    cargo build {{ pure_crates }} --target {{ host_target }}

# build all crates including ESP-IDF (requires espup; does NOT cover rustyfarian-esp-hal-ws2812 — use check-hal)
build-all:
    cargo +esp build --workspace --exclude rustyfarian-esp-hal-ws2812

# check platform-independent crates (no ESP toolchain required)
check:
    cargo check {{ pure_crates }} --target {{ host_target }}

# check all crates including ESP-IDF (requires espup; does NOT cover rustyfarian-esp-hal-ws2812 — use check-hal)
check-all:
    cargo +esp check --workspace --exclude rustyfarian-esp-hal-ws2812

# check only the ESP-IDF driver crate (requires espup)
check-idf:
    cargo +esp check -p rustyfarian-esp-idf-ws2812

# check the esp-hal bare-metal driver (requires: rustup target add riscv32imac-unknown-none-elf)
check-hal:
    cargo check {{ hal_crate }} --target {{ hal_target }}

# --- Examples -------------------------------------------------------------

# build a driver example; driver and chip inferred from {driver}_{chip}_{name} prefix
build-example crate_alias example:
    scripts/build-example.sh "{{ crate_alias }}" "{{ example }}"

# build the ESP32-C3 pulse example (alias for: just build-example hal-ws2812 hal_c3_pulse)
build-example-c3: (build-example "hal-ws2812" "hal_c3_pulse")

# build the ESP32-C6 pulse example (alias for: just build-example hal-ws2812 hal_c6_pulse)
build-example-c6: (build-example "hal-ws2812" "hal_c6_pulse")

# build the Adafruit Feather ESP32 V2 rainbow example (alias for: just build-example idf-ws2812 idf_esp32_rainbow)
build-example-esp32: (build-example "idf-ws2812" "idf_esp32_rainbow")

# build the ESP32-WROOM pulse example (alias for: just build-example hal-ws2812 hal_esp32_pulse)
build-example-esp32-hal: (build-example "hal-ws2812" "hal_esp32_pulse")

# ensure the IDF-built v5.3.3 bootloader is in the build cache for the given chip (c3 or c6)
ensure-bootloader chip:
    scripts/ensure-bootloader.sh "{{ chip }}"

# build and flash a driver example; driver and chip inferred from {driver}_{chip}_{name} prefix
run-example crate_alias example:
    scripts/run-example.sh "{{ crate_alias }}" "{{ example }}"

# --- Flash & Monitor ------------------------------------------------------

# build and flash any example; crate auto-detected from name prefix (hal_* or idf_*)
flash example:
    scripts/flash.sh "{{ example }}"

# build, flash, and open serial monitor — the human workflow
run example: (flash example)
    espflash monitor

# open serial monitor on the connected ESP board (requires espflash)
monitor:
    espflash monitor

# erase the connected ESP device's flash completely (use before reflashing on boot failures)
[confirm]
erase-flash:
    espflash erase-flash

# --- Code Quality ---------------------------------------------------------

# format all code
fmt:
    cargo fmt

# check formatting without modifying files
fmt-check:
    cargo fmt -- --check

# run clippy on platform-independent crates
clippy:
    cargo clippy {{ pure_crates }} --target {{ host_target }} -- -D warnings

# run clippy on all crates including ESP-IDF (requires espup; does NOT cover rustyfarian-esp-hal-ws2812 — use clippy-hal)
clippy-all:
    cargo +esp clippy --workspace --exclude rustyfarian-esp-hal-ws2812 -- -D warnings

# run clippy on only the ESP-IDF driver crate (requires espup)
clippy-idf:
    cargo +esp clippy -p rustyfarian-esp-idf-ws2812 -- -D warnings

# run clippy on the esp-hal bare-metal driver (requires: rustup target add riscv32imac-unknown-none-elf)
clippy-hal:
    cargo clippy {{ hal_crate }} --target {{ hal_target }} -- -D warnings

# run unit and doc tests
test:
    cargo test {{ pure_crates }} --target {{ host_target }}

# run tests with stdout/stderr visible
test-verbose:
    cargo test {{ pure_crates }} --target {{ host_target }} -- --nocapture

# test a specific crate (e.g., just test-crate ferriswheel)
test-crate crate:
    cargo test -p {{ crate }} --target {{ host_target }}

# --- Documentation --------------------------------------------------------

# build rustdoc for platform-independent crates
doc:
    cargo doc {{ pure_crates }} --target {{ host_target }} --no-deps

# build and open docs in browser
doc-open:
    cargo doc {{ pure_crates }} --target {{ host_target }} --no-deps --open

# --- Maintenance ----------------------------------------------------------

# install required development tooling (cargo-deny, cargo-audit, cargo-watch)
setup:
    cargo install cargo-deny cargo-audit cargo-watch

# check dependency licenses, advisories, and bans
deny:
    cargo deny check

# check dependencies for known security vulnerabilities (requires cargo-audit)
audit:
    cargo audit

# Run CI workflow locally via act (requires Docker + act)
act-ci:
    act -j check-and-test

# Run format-check workflow locally via act (requires Docker + act)
act-fmt:
    act -j fmt

# Run clippy workflow locally via act (requires Docker + act)
act-clippy:
    act -j clippy

# Run audit workflow locally via act (requires Docker + act)
act-audit:
    act -j audit

# Run all CI workflows locally via act (requires Docker + act)
act-all: act-fmt act-clippy act-ci act-audit

# update dependencies
update:
    cargo update

# clean build artifacts
clean:
    cargo clean

# clean only the ESP-IDF crate's build artifacts; also removes esp-idf-sys incremental artifacts
# (needed after sdkconfig.defaults changes)
clean-idf:
    cargo clean -p rustyfarian-esp-idf-ws2812
    rm -rf target/riscv32imac-esp-espidf/debug/build/esp-idf-sys-*/
    rm -rf target/riscv32imc-esp-espidf/debug/build/esp-idf-sys-*/
    rm -rf target/xtensa-esp32-espidf/debug/build/esp-idf-sys-*/

# watch and re-run tests on file changes (requires cargo-watch)
watch:
    cargo watch -x "test {{ pure_crates }} --target {{ host_target }}"

# --- Composite ------------------------------------------------------------

# full pre-commit verification: format, check, lint, test (modifies files — local use only)
pre-commit: fmt check clippy test

# verify code quality without modifying files; suggests 'just pre-commit' on formatting issues
verify:
    @cargo fmt -- --check || (printf '\nFormatting issues found — run `just pre-commit` to auto-fix.\n' >&2 && exit 1)
    cargo check {{ pure_crates }} --target {{ host_target }}
    cargo clippy {{ pure_crates }} --target {{ host_target }} -- -D warnings
    cargo test {{ pure_crates }} --target {{ host_target }}

# CI-equivalent verification (non-modifying): format check, deny, check, lint, test
ci: fmt-check deny check clippy test
