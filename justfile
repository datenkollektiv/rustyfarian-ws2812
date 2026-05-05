# Rustyfarian WS2812 — development tasks
#
# The workspace defaults to the ESP32 target (riscv32imac-esp-espidf) via
# .cargo/config.toml, so every recipe that touches platform-independent crates
# explicitly passes --target to override it.

host_target := `scripts/host-target.sh`
pure_crates := "-p bunting -p ferriswheel -p pennant -p rustyfarian-avr-ws2812"
hal_target := "riscv32imac-unknown-none-elf"
hal_crate := "-p rustyfarian-esp-hal-ws2812"
avr_nightly := "nightly-2025-04-27"

# list available recipes (default)
_default:
    @just --list

# --- Build & Check --------------------------------------------------------

# build platform-independent crates
build:
    cargo build {{ pure_crates }} --target {{ host_target }}

# build all crates including ESP-IDF (requires espup; does NOT cover rustyfarian-esp-hal-ws2812 or rustyfarian-avr-ws2812 — use check-hal / check-avr)
build-all:
    cargo +esp build --workspace --exclude rustyfarian-esp-hal-ws2812 --exclude rustyfarian-avr-ws2812

# check platform-independent crates (no ESP toolchain required)
check:
    cargo check {{ pure_crates }} --target {{ host_target }}

# check ferriswheel with the smart-leds-compat feature — exercises the rgb-version-divergence guard
check-ferriswheel-smart-leds-compat:
    cargo check -p ferriswheel --features smart-leds-compat --target {{ host_target }}

# test pennant with the hal feature — exercises the SimpleLed adapter (off by default)
test-pennant-hal:
    cargo test -p pennant --features hal --target {{ host_target }}

# check all crates including ESP-IDF (requires espup; does NOT cover rustyfarian-esp-hal-ws2812 or rustyfarian-avr-ws2812 — use check-hal / check-avr)
check-all:
    cargo +esp check --workspace --exclude rustyfarian-esp-hal-ws2812 --exclude rustyfarian-avr-ws2812

# check only the ESP-IDF driver crate (requires espup)
check-idf:
    cargo +esp check -p rustyfarian-esp-idf-ws2812

# check the esp-hal bare-metal driver (requires: rustup target add riscv32imac-unknown-none-elf)
check-hal:
    cargo check {{ hal_crate }} --target {{ hal_target }}

# check the AVR SPI driver on the host target (no AVR toolchain required)
check-avr:
    cargo check -p rustyfarian-avr-ws2812 --target {{ host_target }}

# check the AVR driver on the host with all features (bitbang + smart-leds-trait)
check-avr-all-features:
    cargo check -p rustyfarian-avr-ws2812 --features bitbang,smart-leds-trait --target {{ host_target }}

# test the AVR driver on the host with all features
test-avr-all-features:
    cargo test -p rustyfarian-avr-ws2812 --features bitbang,smart-leds-trait --target {{ host_target }}

# check the AVR SPI driver against the real avr-none target (requires: just setup-avr, avr-gcc)
check-avr-target:
    RUSTFLAGS="-C target-cpu=atmega328p" cargo +{{ avr_nightly }} check -p rustyfarian-avr-ws2812 --target avr-none -Z build-std=core

# check the AVR bit-bang driver against the real avr-none target (requires: just setup-avr, avr-gcc)
check-avr-target-bitbang:
    RUSTFLAGS="-C target-cpu=atmega328p" cargo +{{ avr_nightly }} check -p rustyfarian-avr-ws2812 --features bitbang --target avr-none -Z build-std=core

# --- AVR Examples ---------------------------------------------------------

# build the AVR Nano rainbow example (default = bit-bang backend; requires: just setup-avr, avr-gcc)
build-avr-example:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} build --release -Z build-std=core

# build every binary in the AVR Nano example crate (default + bitbang_demo + spi_rainbow + bitbang_spike)
build-avr-example-all-bins:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} build --release -Z build-std=core --bins

# build and flash the AVR Nano rainbow demo — bit-bang backend, recommended (requires: just setup-avr, avr-gcc, ravedude)
flash-avr-example:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} run --release -Z build-std=core

# build and flash the production bit-bang PulseEffect demo (uses Ws2812BitBang from the driver crate)
flash-avr-bitbang-demo:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} run --release -Z build-std=core --bin bitbang_demo

# build and flash the SPI prerendered rainbow — DIAGNOSTIC ONLY; many strips render this as white-ish (see ADR 007)
flash-avr-spi-rainbow:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} run --release -Z build-std=core --bin spi_rainbow

# build and flash the AVR Nano bit-bang spike (frozen low-level reference, see docs/features/avr-bitbang-driver.md)
flash-avr-bitbang-spike:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} run --release -Z build-std=core --bin bitbang_spike

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

# install all required targets and development tooling
setup: setup-tools setup-hal setup-avr

# install cargo development tools (cargo-deny, cargo-audit, cargo-watch)
setup-tools:
    cargo install cargo-deny cargo-audit cargo-watch

# install the bare-metal RISC-V target for esp-hal driver (ESP32-C6, ESP32-C3)
setup-hal:
    rustup target add riscv32imac-unknown-none-elf
    rustup target add riscv32imc-unknown-none-elf

# install the AVR nightly toolchain with rust-src for build-std (requires: avr-gcc via system package manager)
setup-avr:
    rustup toolchain install {{ avr_nightly }}
    rustup component add rust-src --toolchain {{ avr_nightly }}
    @echo "AVR toolchain ready: {{ avr_nightly }}"
    @echo "Ensure avr-gcc is installed: brew install avr-gcc (macOS) / apt install gcc-avr (Debian)"

# install ESP-IDF toolchain via espup (requires: espup already installed)
setup-esp:
    @echo "ESP-IDF toolchain is managed by espup. Install it first:"
    @echo "  cargo install espup"
    @echo "  espup install"
    @echo "Then use 'cargo +esp' for ESP-IDF builds."

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
    cargo test -p pennant --features hal --target {{ host_target }}

# CI-equivalent verification (non-modifying): format check, deny, check, lint, test
ci: fmt-check deny check clippy test

# --- Release --------------------------------------------------------------
#
# First-wave publish targets `bunting`, `pennant`, and `ferriswheel`. The three
# crates do not depend on each other, so any order works — but the canonical
# order recorded in `docs/features/crates-io-publication-v1.md` is
# bunting → pennant → ferriswheel.
#
# `cargo publish --dry-run` packages the crate and runs verification on the
# host target without uploading. The host-target override is required because
# the workspace defaults to an ESP cross-compile target via `.cargo/config.toml`.

# verify all v1 publish-target crates package cleanly (no upload)
release-dry-run: verify
    cargo publish --dry-run -p bunting --target {{ host_target }}
    cargo publish --dry-run -p pennant --target {{ host_target }}
    cargo publish --dry-run -p ferriswheel --target {{ host_target }}

# verify one crate packages cleanly (no upload). Use: just release-dry-run-crate bunting
release-dry-run-crate crate:
    cargo publish --dry-run -p {{ crate }} --target {{ host_target }}

# publish one crate to crates.io (dep order: bunting → pennant → ferriswheel). Use: just release-publish bunting
[confirm]
release-publish crate:
    cargo publish -p {{ crate }} --target {{ host_target }}
