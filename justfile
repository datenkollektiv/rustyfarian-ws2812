# Rustyfarian WS2812 — development tasks
#
# The workspace defaults to the bare-metal ESP target (riscv32imac-unknown-none-elf)
# via .cargo/config.toml. IDF recipes explicitly pass --target {{ idf_target }}.
# Pure-crate recipes explicitly pass --target {{ host_target }}.

host_target := `scripts/host-target.sh`
pure_crates := "-p bunting -p ferriswheel -p pennant -p rustyfarian-avr-ws2812"
hal_target := "riscv32imac-unknown-none-elf"
idf_target := "riscv32imac-esp-espidf"
hal_crate := "-p rustyfarian-esp-hal-ws2812"
avr_nightly := "nightly-2025-04-27"

ramdisk := "/Volumes/RustBuilds"
# Detect a real mounted volume; `path_exists` would be fooled by a stale /Volumes/<name>
# directory that was never `hdiutil attach`-ed (or wasn't cleaned up after detach).
# The wrapper delegates to `is_ramdisk_mounted` in scripts/lib.sh so the check lives in
# one place and stays consistent with scripts/ramdisk.sh.
ramdisk_mounted := shell(justfile_directory() + '/scripts/ramdisk-mounted.sh "' + ramdisk + '"')
hal_dir  := if ramdisk_mounted == "true" { ramdisk + "/targets/hal/" + file_name(justfile_directory()) } else { "target/hal" }
idf_dir  := if ramdisk_mounted == "true" { ramdisk + "/targets/idf/" + file_name(justfile_directory()) } else { "target/idf" }

# list available recipes (default)
_default:
    @just --list

# --- Build Environment ----------------------------------------------------

# show RAM disk status, resolved target dirs, and sccache
[group('Build Environment')]
doctor:
    @scripts/doctor.sh "{{ramdisk}}" "{{hal_dir}}" "{{idf_dir}}"

# manage the RAM disk: just ramdisk attach | detach
[group('Build Environment')]
ramdisk action:
    @scripts/ramdisk.sh "{{action}}"

# --- Build & Check --------------------------------------------------------

# build platform-independent crates
[group('Build & Check')]
build:
    cargo build {{ pure_crates }} --target {{ host_target }}

# build all crates including ESP-IDF (requires espup; does NOT cover rustyfarian-esp-hal-ws2812 or rustyfarian-avr-ws2812 — use check-hal / check-avr)
[group('Build & Check')]
build-all:
    cargo +esp build --workspace --exclude rustyfarian-esp-hal-ws2812 --exclude rustyfarian-avr-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }}

# check platform-independent crates (no ESP toolchain required)
[group('Build & Check')]
check:
    cargo check {{ pure_crates }} --target {{ host_target }}

# check all crates including ESP-IDF (requires espup; does NOT cover rustyfarian-esp-hal-ws2812 or rustyfarian-avr-ws2812 — use check-hal / check-avr)
[group('Build & Check')]
check-all:
    cargo +esp check --workspace --exclude rustyfarian-esp-hal-ws2812 --exclude rustyfarian-avr-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }}

# check only the ESP-IDF driver crate (requires espup)
[group('Build & Check')]
check-idf:
    cargo +esp check -p rustyfarian-esp-idf-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }}

# check the esp-hal bare-metal driver (requires: rustup target add riscv32imac-unknown-none-elf)
[group('Build & Check')]
check-hal:
    cargo check {{ hal_crate }} --target {{ hal_target }} --target-dir {{ hal_dir }}

# check the esp-hal bare-metal driver on the Xtensa ESP32 target (requires: just setup esp)
[group('Build & Check')]
check-hal-xtensa:
    cargo +esp check {{ hal_crate }} --target xtensa-esp32-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32,unstable -Z build-std=core
    cargo +esp check {{ hal_crate }} --target xtensa-esp32-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32,unstable,rt --examples -Z build-std=core

# check the AVR SPI driver on the host target (no AVR toolchain required)
[group('Build & Check')]
check-avr:
    cargo check -p rustyfarian-avr-ws2812 --target {{ host_target }}

# check the AVR driver on the host with all features (bitbang + smart-leds-trait)
[group('Build & Check')]
check-avr-all-features:
    cargo check -p rustyfarian-avr-ws2812 --features bitbang,smart-leds-trait --target {{ host_target }}

# check the AVR SPI driver against the real avr-none target (requires: just setup avr, avr-gcc)
[group('Build & Check')]
check-avr-target:
    RUSTFLAGS="-C target-cpu=atmega328p" cargo +{{ avr_nightly }} check -p rustyfarian-avr-ws2812 --target avr-none -Z build-std=core

# check the AVR bit-bang driver against the real avr-none target (requires: just setup avr, avr-gcc)
[group('Build & Check')]
check-avr-target-bitbang:
    RUSTFLAGS="-C target-cpu=atmega328p" cargo +{{ avr_nightly }} check -p rustyfarian-avr-ws2812 --features bitbang --target avr-none -Z build-std=core

# check ferriswheel with the smart-leds-compat feature — exercises the rgb-version-divergence guard
[group('Build & Check')]
check-ferriswheel-smart-leds-compat:
    cargo check -p ferriswheel --features smart-leds-compat --target {{ host_target }}

# --- Test & Lint ----------------------------------------------------------

# run unit and doc tests
[group('Test & Lint')]
test:
    cargo test {{ pure_crates }} --target {{ host_target }}

# run tests with stdout/stderr visible
[group('Test & Lint')]
test-verbose:
    cargo test {{ pure_crates }} --target {{ host_target }} -- --nocapture

# test a specific crate (e.g., just test-crate ferriswheel)
[group('Test & Lint')]
test-crate crate:
    cargo test -p {{ crate }} --target {{ host_target }}

# test pennant with the hal feature — exercises the SimpleLed adapter (off by default)
[group('Test & Lint')]
test-pennant-hal:
    cargo test -p pennant --features hal --target {{ host_target }}

# test the AVR driver on the host with all features
[group('Test & Lint')]
test-avr-all-features:
    cargo test -p rustyfarian-avr-ws2812 --features bitbang,smart-leds-trait --target {{ host_target }}

# format all code
[group('Test & Lint')]
fmt:
    cargo fmt

# check formatting without modifying files
[group('Test & Lint')]
fmt-check:
    cargo fmt --all -- --check

# run clippy on platform-independent crates
[group('Test & Lint')]
clippy:
    cargo clippy {{ pure_crates }} --target {{ host_target }} -- -D warnings

# run clippy on all crates applicable to the ESP-IDF-target lint pass (requires espup); not literally every workspace crate — excludes rustyfarian-esp-hal-ws2812 (use clippy-hal) and rustyfarian-avr-ws2812 (linted on the host target via clippy)
[group('Test & Lint')]
clippy-all:
    cargo +esp clippy --workspace --exclude rustyfarian-esp-hal-ws2812 --exclude rustyfarian-avr-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }} -- -D warnings

# run clippy on only the ESP-IDF driver crate (requires espup)
[group('Test & Lint')]
clippy-idf:
    cargo +esp clippy -p rustyfarian-esp-idf-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }} -- -D warnings

# run clippy on the esp-hal bare-metal driver (requires: rustup target add riscv32imac-unknown-none-elf)
[group('Test & Lint')]
clippy-hal:
    cargo clippy {{ hal_crate }} --target {{ hal_target }} --target-dir {{ hal_dir }} -- -D warnings

# check dependency licenses, advisories, and bans
[group('Test & Lint')]
deny:
    cargo deny check

# check dependencies for known security vulnerabilities (requires cargo-audit)
[group('Test & Lint')]
audit:
    cargo audit

# --- AVR Examples ---------------------------------------------------------

# build the AVR Nano rainbow example (default = bit-bang backend; requires: just setup avr, avr-gcc)
[group('AVR Examples')]
build-avr-example:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} build --release -Z build-std=core

# build every binary in the AVR Nano example crate (default + bitbang_demo + spi_rainbow + bitbang_spike)
[group('AVR Examples')]
build-avr-example-all-bins:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} build --release -Z build-std=core --bins

# build and flash the AVR Nano rainbow demo — bit-bang backend, recommended (requires: just setup avr, avr-gcc, ravedude)
[group('AVR Examples')]
flash-avr-example:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} run --release -Z build-std=core

# build and flash the production bit-bang PulseEffect demo (uses Ws2812BitBang from the driver crate)
[group('AVR Examples')]
flash-avr-bitbang-demo:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} run --release -Z build-std=core --bin bitbang_demo

# build and flash the SPI prerendered rainbow — DIAGNOSTIC ONLY; many strips render this as white-ish (see ADR 007)
[group('AVR Examples')]
flash-avr-spi-rainbow:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} run --release -Z build-std=core --bin spi_rainbow

# build and flash the AVR Nano bit-bang spike (frozen low-level reference, see docs/features/avr-bitbang-driver.md)
[group('AVR Examples')]
flash-avr-bitbang-spike:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} run --release -Z build-std=core --bin bitbang_spike

# --- ESP Examples ---------------------------------------------------------

# build a driver example; driver and chip inferred from {driver}_{chip}_{name} prefix
[group('ESP Examples')]
build-example crate_alias example:
    scripts/build-example.sh "{{ crate_alias }}" "{{ example }}" "{{ hal_dir }}" "{{ idf_dir }}"

# build the ESP32-C3 pulse example (alias for: just build-example hal-ws2812 hal_c3_pulse)
[group('ESP Examples')]
build-example-c3: (build-example "hal-ws2812" "hal_c3_pulse")

# build the ESP32-C6 pulse example (alias for: just build-example hal-ws2812 hal_c6_pulse)
[group('ESP Examples')]
build-example-c6: (build-example "hal-ws2812" "hal_c6_pulse")

# build the Adafruit Feather ESP32 V2 rainbow example (alias for: just build-example idf-ws2812 idf_esp32_rainbow)
[group('ESP Examples')]
build-example-esp32: (build-example "idf-ws2812" "idf_esp32_rainbow")

# build the ESP32-WROOM pulse example (alias for: just build-example hal-ws2812 hal_esp32_pulse)
[group('ESP Examples')]
build-example-esp32-hal: (build-example "hal-ws2812" "hal_esp32_pulse")

# ensure the IDF-built v5.3.3 bootloader is in the build cache for the given chip (c3 or c6)
[group('ESP Examples')]
ensure-bootloader chip:
    scripts/ensure-bootloader.sh "{{ chip }}" "{{ hal_dir }}" "{{ idf_dir }}"

# build and flash a driver example; driver and chip inferred from {driver}_{chip}_{name} prefix
[group('ESP Examples')]
run-example crate_alias example:
    scripts/run-example.sh "{{ crate_alias }}" "{{ example }}" "{{ hal_dir }}" "{{ idf_dir }}"

# --- Flash & Monitor ------------------------------------------------------

# build and flash any example; crate auto-detected from name prefix (hal_* or idf_*)
[group('Flash & Monitor')]
flash example:
    scripts/flash.sh "{{ example }}" "{{ hal_dir }}" "{{ idf_dir }}"

# build, flash, and open serial monitor — the human workflow
[group('Flash & Monitor')]
run example: (flash example)
    espflash monitor

# open serial monitor on the connected ESP board (requires espflash)
[group('Flash & Monitor')]
monitor:
    espflash monitor

# erase the connected ESP device's flash completely (use before reflashing on boot failures)
[group('Flash & Monitor')]
[confirm]
erase-flash:
    espflash erase-flash

# --- Documentation --------------------------------------------------------

# build rustdoc for platform-independent crates
[group('Documentation')]
doc:
    cargo doc {{ pure_crates }} --target {{ host_target }} --no-deps

# build and open docs in browser
[group('Documentation')]
doc-open:
    cargo doc {{ pure_crates }} --target {{ host_target }} --no-deps --open

# --- CI -------------------------------------------------------------------

# verify code quality without modifying files; suggests 'just pre-commit' on formatting issues
[group('CI')]
verify:
    @cargo fmt --all -- --check || (printf '\nFormatting issues found — run `just pre-commit` to auto-fix.\n' >&2 && exit 1)
    cargo check {{ pure_crates }} --target {{ host_target }}
    cargo clippy {{ pure_crates }} --target {{ host_target }} -- -D warnings
    cargo test {{ pure_crates }} --target {{ host_target }}
    cargo test -p pennant --features hal --target {{ host_target }}

# full pre-commit verification: format, check, lint, test (modifies files — local use only)
[group('CI')]
pre-commit: fmt check clippy test

# CI-equivalent verification (non-modifying): format check, deny, check, lint, test
[group('CI')]
ci: fmt-check deny check clippy test

# Run CI workflow locally via act (requires Docker + act)
[group('CI')]
act-ci:
    act -j check-and-test

# Run format-check workflow locally via act (requires Docker + act)
[group('CI')]
act-fmt:
    act -j fmt

# Run clippy workflow locally via act (requires Docker + act)
[group('CI')]
act-clippy:
    act -j clippy

# Run audit workflow locally via act (requires Docker + act)
[group('CI')]
act-audit:
    act -j audit

# Run all CI workflows locally via act (requires Docker + act)
[group('CI')]
act-all: act-fmt act-clippy act-ci act-audit

# --- Setup ----------------------------------------------------------------

# install components: just setup [tools|hal|avr|esp|all]
[group('Setup')]
setup component="all":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ component }}" in
        tools)
            cargo install cargo-deny cargo-audit cargo-watch espup
            ;;
        hal)
            rustup target add riscv32imac-unknown-none-elf
            rustup target add riscv32imc-unknown-none-elf
            ;;
        avr)
            rustup toolchain install {{ avr_nightly }}
            rustup component add rust-src --toolchain {{ avr_nightly }}
            echo "AVR toolchain ready: {{ avr_nightly }}"
            echo "Ensure avr-gcc is installed: brew install avr-gcc (macOS) / apt install gcc-avr (Debian)"
            ;;
        esp)
            espup install
            ;;
        all)
            cargo install cargo-deny cargo-audit cargo-watch espup
            rustup target add riscv32imac-unknown-none-elf
            rustup target add riscv32imc-unknown-none-elf
            rustup toolchain install {{ avr_nightly }}
            rustup component add rust-src --toolchain {{ avr_nightly }}
            echo "AVR toolchain ready: {{ avr_nightly }}"
            echo "Ensure avr-gcc is installed: brew install avr-gcc (macOS) / apt install gcc-avr (Debian)"
            echo ""
            echo "For Xtensa/ESP-IDF support, run: just setup esp"
            ;;
        *)
            echo "error: unknown component '{{ component }}'" >&2
            echo "usage: just setup [tools|hal|avr|esp|all]" >&2
            exit 1
            ;;
    esac

# --- Maintenance ----------------------------------------------------------

# update dependencies
[group('Maintenance')]
update:
    cargo update

# clean build artifacts (target/ide, hal, idf, and avr example target dirs)
[group('Maintenance')]
clean:
    cargo clean --target-dir target/ide
    cargo clean --target-dir "{{ hal_dir }}"
    cargo clean --target-dir "{{ idf_dir }}"
    rm -rf examples/avr-nano-rainbow/target

# clean ESP-IDF crate artifacts and esp-idf-sys hash dirs (needed after sdkconfig.defaults changes or Cargo.toml edits)
[group('Maintenance')]
clean-idf:
    cargo clean -p rustyfarian-esp-idf-ws2812 --target-dir "{{ idf_dir }}"
    rm -rf "{{ idf_dir }}"/riscv32imac-esp-espidf/debug/build/esp-idf-sys-*/
    rm -rf "{{ idf_dir }}"/riscv32imc-esp-espidf/debug/build/esp-idf-sys-*/
    rm -rf "{{ idf_dir }}"/xtensa-esp32-espidf/debug/build/esp-idf-sys-*/

# watch and re-run tests on file changes (requires cargo-watch)
[group('Maintenance')]
watch:
    cargo watch -x "test {{ pure_crates }} --target {{ host_target }}"

# --- Release --------------------------------------------------------------

# verify pure-logic crates package cleanly (no upload); driver crates require pure 0.X live on crates.io first — use release-dry-run-crate after Stage 1 publish
[group('Release')]
release-dry-run: verify
    cargo publish --dry-run -p bunting --target {{ host_target }}
    cargo publish --dry-run -p pennant --target {{ host_target }}
    cargo publish --dry-run -p ferriswheel --target {{ host_target }}

# verify one pure/AVR crate packages cleanly against host target (no upload). Use: just release-dry-run-crate rustyfarian-avr-ws2812
[group('Release')]
release-dry-run-crate crate:
    cargo publish --dry-run -p {{ crate }} --target {{ host_target }}

# verify esp-hal driver packages cleanly against bare-metal target (no upload; requires riscv32imac-unknown-none-elf target installed)
[group('Release')]
release-dry-run-hal:
    cargo publish --dry-run -p rustyfarian-esp-hal-ws2812 --target {{ hal_target }}

# verify esp-idf driver packages cleanly against IDF target (no upload; requires espup)
[group('Release')]
release-dry-run-idf:
    cargo +esp publish --dry-run -p rustyfarian-esp-idf-ws2812 --target riscv32imac-esp-espidf --target-dir {{ idf_dir }}

# publish one pure/AVR crate to crates.io. Use: just release-publish bunting
[group('Release')]
[confirm]
release-publish crate:
    cargo publish -p {{ crate }} --target {{ host_target }}

# publish esp-hal driver to crates.io (requires riscv32imac-unknown-none-elf target installed)
[group('Release')]
[confirm]
release-publish-hal:
    cargo publish -p rustyfarian-esp-hal-ws2812 --target {{ hal_target }}

# publish esp-idf driver to crates.io (requires espup)
[group('Release')]
[confirm]
release-publish-idf:
    cargo +esp publish -p rustyfarian-esp-idf-ws2812 --target riscv32imac-esp-espidf --target-dir {{ idf_dir }}
