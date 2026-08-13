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
hal_dir := if ramdisk_mounted == "true" { ramdisk + "/targets/hal/" + file_name(justfile_directory()) } else { "target/hal" }
idf_dir := if ramdisk_mounted == "true" { ramdisk + "/targets/idf/" + file_name(justfile_directory()) } else { "target/idf" }

# list available recipes (default)
_default:
    @just --list

# --- Build Environment ----------------------------------------------------

# show RAM disk status, resolved target dirs, and sccache
[group('Build Environment')]
doctor:
    @scripts/doctor.sh "{{ ramdisk }}" "{{ hal_dir }}" "{{ idf_dir }}" "$(scripts/idf-build-dir.sh)" "$(scripts/idf-build-dir.sh --glob)"

# manage the RAM disk: just ramdisk attach | detach
[group('Build Environment')]
ramdisk action:
    @scripts/ramdisk.sh "{{ action }}"

# --- Build & Check --------------------------------------------------------

# build platform-independent crates
[group('Build & Check')]
build:
    cargo build {{ pure_crates }} --target {{ host_target }}

# build all crates including ESP-IDF (requires espup; does NOT cover rustyfarian-esp-hal-ws2812 or rustyfarian-avr-ws2812 — use check-hal / check-avr)
# build every crate the ESP-IDF toolchain compiles
[group('Build & Check')]
build-all:
    cargo +esp build --workspace --exclude rustyfarian-esp-hal-ws2812 --exclude rustyfarian-avr-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }} $(scripts/idf-build-dir.sh --config)

# check platform-independent crates (no ESP toolchain)
[group('Build & Check')]
check:
    cargo check {{ pure_crates }} --target {{ host_target }}

# check all crates including ESP-IDF (requires espup; does NOT cover rustyfarian-esp-hal-ws2812 or rustyfarian-avr-ws2812 — use check-hal / check-avr)
# check every crate the ESP-IDF toolchain compiles
[group('Build & Check')]
check-all:
    cargo +esp check --workspace --exclude rustyfarian-esp-hal-ws2812 --exclude rustyfarian-avr-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }} $(scripts/idf-build-dir.sh --config)

# check only the ESP-IDF driver crate (requires espup)
[group('Build & Check')]
check-idf:
    cargo +esp check -p rustyfarian-esp-idf-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }} $(scripts/idf-build-dir.sh --config)

# check the esp-hal bare-metal driver (requires: rustup target add riscv32imac-unknown-none-elf)
# check the esp-hal bare-metal driver (RISC-V target)
[group('Build & Check')]
check-hal:
    cargo check {{ hal_crate }} --target {{ hal_target }} --target-dir {{ hal_dir }}

# Library pass, then an --examples pass with `rt` so hal_c3_pulse is compiled too. The C6/imac
# default set is covered by check-hal (requires: rustup target add riscv32imc-unknown-none-elf)
# check the esp-hal driver on the ESP32-C3 target
[group('Build & Check')]
check-hal-c3:
    cargo check {{ hal_crate }} --target riscv32imc-unknown-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32c3,unstable,pennant
    cargo check {{ hal_crate }} --target riscv32imc-unknown-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32c3,unstable,pennant,rt --examples

# check the esp-hal bare-metal driver on the Xtensa ESP32 target (requires: just setup esp)
# check the esp-hal driver on the Xtensa ESP32 target
[group('Build & Check')]
check-hal-xtensa:
    cargo +esp check {{ hal_crate }} --target xtensa-esp32-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32,unstable,pennant -Z build-std=core
    cargo +esp check {{ hal_crate }} --target xtensa-esp32-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32,unstable,pennant,rt --examples -Z build-std=core

# check the AVR SPI driver on host (no AVR toolchain)
[group('Build & Check')]
check-avr:
    cargo check -p rustyfarian-avr-ws2812 --target {{ host_target }}

# check the AVR driver on host with all features
[group('Build & Check')]
check-avr-all-features:
    cargo check -p rustyfarian-avr-ws2812 --features bitbang,smart-leds-trait --target {{ host_target }}

# check the AVR SPI driver against the real avr-none target (requires: just setup avr, avr-gcc)
# check the AVR SPI driver on the real avr-none target
[group('Build & Check')]
check-avr-target:
    RUSTFLAGS="-C target-cpu=atmega328p" cargo +{{ avr_nightly }} check -p rustyfarian-avr-ws2812 --target avr-none -Z build-std=core

# check the AVR bit-bang driver against the real avr-none target (requires: just setup avr, avr-gcc)
# check the AVR bit-bang driver on the avr-none target
[group('Build & Check')]
check-avr-target-bitbang:
    RUSTFLAGS="-C target-cpu=atmega328p" cargo +{{ avr_nightly }} check -p rustyfarian-avr-ws2812 --features bitbang --target avr-none -Z build-std=core

# check ferriswheel with the smart-leds-compat feature — exercises the rgb-version-divergence guard
# check ferriswheel's smart-leds-compat feature
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
# test pennant's hal feature (SimpleLed adapter)
[group('Test & Lint')]
test-pennant-hal:
    cargo test -p pennant --features hal --target {{ host_target }}

# run clippy on pennant's hal feature — covers the SimpleLed / RgbGpioLed / RgbPwmLed adapters (off by default)
# clippy pennant's hal feature (LED adapters)
[group('Test & Lint')]
clippy-pennant-hal:
    cargo clippy -p pennant --features hal --target {{ host_target }} -- -D warnings

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
# clippy every crate the ESP-IDF toolchain lints
[group('Test & Lint')]
clippy-all:
    cargo +esp clippy --workspace --exclude rustyfarian-esp-hal-ws2812 --exclude rustyfarian-avr-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }} $(scripts/idf-build-dir.sh --config) -- -D warnings

# clippy the ESP-IDF driver crate only (requires espup)
[group('Test & Lint')]
clippy-idf:
    cargo +esp clippy -p rustyfarian-esp-idf-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }} $(scripts/idf-build-dir.sh --config) -- -D warnings

# run clippy on the esp-hal bare-metal driver (requires: rustup target add riscv32imac-unknown-none-elf)
# clippy the esp-hal bare-metal driver
[group('Test & Lint')]
clippy-hal:
    cargo clippy {{ hal_crate }} --target {{ hal_target }} --target-dir {{ hal_dir }} -- -D warnings

# check dependency licenses, advisories, and bans
[group('Test & Lint')]
deny:
    cargo deny check

# scan deps for vulnerabilities (requires cargo-audit)
[group('Test & Lint')]
audit:
    cargo audit

# --- AVR Examples ---------------------------------------------------------

# build the AVR Nano rainbow example (default = bit-bang backend; requires: just setup avr, avr-gcc)
# build the AVR Nano rainbow example (bit-bang)
[group('AVR Examples')]
build-avr-example:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} build --release -Z build-std=core

# build every binary in the AVR Nano example crate (default + bitbang_demo + spi_rainbow + bitbang_spike).
# --locked is the CI reproducibility gate: it fails loudly if Cargo.toml was edited without
# committing the regenerated Cargo.lock. Local dev recipes deliberately omit it.
# build every binary in the AVR Nano example crate
[group('AVR Examples')]
build-avr-example-all-bins:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} build --release -Z build-std=core --bins --locked

# build the AVR example against upstream avr-hal main instead of the pinned rev — the weekly
# early-warning check. Works on a throwaway copy in a temp dir, so no tracked file is touched
# even transiently; the `crates` symlink keeps the example's ../../crates path deps resolving.
# build the AVR example against upstream avr-hal main
[group('AVR Examples')]
build-avr-example-upstream:
    #!/usr/bin/env bash
    set -euo pipefail
    root="{{ justfile_directory() }}"
    probe="$(mktemp -d)"
    trap 'rm -rf "$probe"' EXIT
    mkdir -p "$probe/examples"
    cp -R "$root/examples/avr-nano-rainbow" "$probe/examples/avr-nano-rainbow"
    rm -rf "$probe/examples/avr-nano-rainbow/target"
    ln -s "$root/crates" "$probe/crates"
    ln -s "$root/Cargo.toml" "$probe/Cargo.toml"
    cd "$probe/examples/avr-nano-rainbow"
    sed -i.bak '/^rev = /d' Cargo.toml && rm -f Cargo.toml.bak
    rm -f Cargo.lock
    cargo +{{ avr_nightly }} build --release -Z build-std=core --bins

# build and flash the AVR Nano rainbow demo — bit-bang backend, recommended (requires: just setup avr, avr-gcc, ravedude)
# flash the AVR Nano rainbow demo (bit-bang)
[group('AVR Examples')]
flash-avr-example:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} run --release -Z build-std=core

# build and flash the production bit-bang PulseEffect demo (uses Ws2812BitBang from the driver crate)
# flash the production bit-bang PulseEffect demo
[group('AVR Examples')]
flash-avr-bitbang-demo:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} run --release -Z build-std=core --bin bitbang_demo

# build and flash the SPI prerendered rainbow — DIAGNOSTIC ONLY; many strips render this as white-ish (see ADR 007)
# flash the SPI prerendered rainbow (DIAGNOSTIC ONLY)
[group('AVR Examples')]
flash-avr-spi-rainbow:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} run --release -Z build-std=core --bin spi_rainbow

# build and flash the AVR Nano bit-bang spike (frozen low-level reference, see docs/features/avr-bitbang-driver.md)
# flash the AVR bit-bang spike (frozen reference)
[group('AVR Examples')]
flash-avr-bitbang-spike:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} run --release -Z build-std=core --bin bitbang_spike

# --- ESP Examples ---------------------------------------------------------

# build a driver example; driver and chip inferred from {driver}_{chip}_{name} prefix
# build a driver example; crate inferred from name
[group('ESP Examples')]
build-example crate_alias example:
    scripts/build-example.sh "{{ crate_alias }}" "{{ example }}" "{{ hal_dir }}" "{{ idf_dir }}"

# build the ESP32-C3 pulse example (alias for: just build-example hal-ws2812 hal_c3_pulse)
# build the ESP32-C3 pulse example
[group('ESP Examples')]
build-example-c3: (build-example "hal-ws2812" "hal_c3_pulse")

# build the ESP32-C6 pulse example (alias for: just build-example hal-ws2812 hal_c6_pulse)
# build the ESP32-C6 pulse example
[group('ESP Examples')]
build-example-c6: (build-example "hal-ws2812" "hal_c6_pulse")

# build the Adafruit Feather ESP32 V2 rainbow example (alias for: just build-example idf-ws2812 idf_esp32_rainbow)
# build the Feather ESP32 V2 rainbow example
[group('ESP Examples')]
build-example-esp32: (build-example "idf-ws2812" "idf_esp32_rainbow")

# build the ESP32-WROOM pulse example (alias for: just build-example hal-ws2812 hal_esp32_pulse)
# build the ESP32-WROOM pulse example (esp-hal)
[group('ESP Examples')]
build-example-esp32-hal: (build-example "hal-ws2812" "hal_esp32_pulse")

# build the ESP32-WROOM discrete RGB LED (pennant RgbGpioLed) example (alias for: just build-example idf-ws2812 idf_esp32_rgb_cycle)
# build the ESP32-WROOM discrete RGB LED example
[group('ESP Examples')]
build-example-esp32-rgb: (build-example "idf-ws2812" "idf_esp32_rgb_cycle")

# build the ESP32-WROOM discrete RGB LED smooth PWM pulse (pennant RgbPwmLed) example (alias for: just build-example idf-ws2812 idf_esp32_rgb_pulse)
# build the ESP32-WROOM RGB PWM pulse example
[group('ESP Examples')]
build-example-esp32-rgb-pulse: (build-example "idf-ws2812" "idf_esp32_rgb_pulse")

# ensure the IDF-built v5.3.3 bootloader is in the build cache for the given chip (c3 or c6)
# cache the IDF-built bootloader for a chip (c3|c6)
[group('ESP Examples')]
ensure-bootloader chip:
    scripts/ensure-bootloader.sh "{{ chip }}" "{{ hal_dir }}" "{{ idf_dir }}"

# build and flash a driver example; driver and chip inferred from {driver}_{chip}_{name} prefix
# build and flash a driver example, then monitor
[group('ESP Examples')]
run-example crate_alias example:
    scripts/run-example.sh "{{ crate_alias }}" "{{ example }}" "{{ hal_dir }}" "{{ idf_dir }}"

# --- Flash & Monitor ------------------------------------------------------

# build and flash any example; crate auto-detected from name prefix (hal_* or idf_*)
# build and flash any example (crate auto-detected)
[group('Flash & Monitor')]
flash example:
    scripts/flash.sh "{{ example }}" "{{ hal_dir }}" "{{ idf_dir }}"

# build, flash, and monitor — the human workflow
[group('Flash & Monitor')]
run example: (flash example)
    espflash monitor

# open serial monitor on the ESP board (requires espflash)
[group('Flash & Monitor')]
monitor:
    espflash monitor

# erase the connected ESP device's flash completely (use before reflashing on boot failures)
# erase the connected ESP device's flash completely
[confirm]
[group('Flash & Monitor')]
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
# check code quality without modifying files
[group('CI')]
verify:
    @cargo fmt --all -- --check || (printf '\nFormatting issues found — run `just pre-commit` to auto-fix.\n' >&2 && exit 1)
    cargo check {{ pure_crates }} --target {{ host_target }}
    cargo clippy {{ pure_crates }} --target {{ host_target }} -- -D warnings
    cargo test {{ pure_crates }} --target {{ host_target }}
    cargo test -p pennant --features hal --target {{ host_target }}
    cargo clippy -p pennant --features hal --target {{ host_target }} -- -D warnings

# full pre-commit verification: format, check, lint, test (modifies files — local use only)
# run format, check, lint, test (modifies files)
[group('CI')]
pre-commit: fmt check clippy test

# CI-equivalent verification (non-modifying): format check, deny, check, lint, test
# CI-equivalent gate: fmt-check, deny, check, lint, test
[group('CI')]
ci: fmt-check deny check clippy test

# run the CI workflow via act (requires Docker)
[group('CI')]
act-ci:
    act -j check-and-test

# run the fmt-check workflow via act (requires Docker)
[group('CI')]
act-fmt:
    act -j fmt

# run the clippy workflow via act (requires Docker)
[group('CI')]
act-clippy:
    act -j clippy

# run the audit workflow via act (requires Docker)
[group('CI')]
act-audit:
    act -j audit

# run all CI workflows via act (requires Docker)
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

# clean build artifacts (ide, hal, idf, avr target dirs)
[group('Maintenance')]
clean:
    cargo clean --target-dir target/ide
    cargo clean --target-dir "{{ hal_dir }}"
    cargo clean --target-dir "{{ idf_dir }}"
    rm -rf examples/avr-nano-rainbow/target

# clean ESP-IDF crate artifacts and esp-idf-sys hash dirs (needed after sdkconfig.defaults changes or Cargo.toml edits)
# clean ESP-IDF artifacts and esp-idf-sys hash dirs
[group('Maintenance')]
clean-idf:
    cargo clean -p rustyfarian-esp-idf-ws2812 --target-dir "{{ idf_dir }}"
    @scripts/idf-cache.sh clean "{{ idf_dir }}"

# drop superseded esp-idf-sys build dirs, keeping the newest per target (fixes "multiple IDF-built bootloaders")
# drop superseded esp-idf-sys dirs, keep newest per target
[group('Maintenance')]
clean-idf-stale:
    @scripts/idf-cache.sh clean-stale

# remove the persistent IDF build-dir cache entirely (the relocated esp-idf-sys CMake trees)
# remove the persistent IDF build-dir cache entirely
[group('Maintenance')]
clean-idf-cache:
    @scripts/idf-cache.sh clean-all

# show the resolved IDF build-dir configuration
[group('Maintenance')]
idf-build-dir-info:
    @scripts/idf-cache.sh info

# watch files and re-run tests (requires cargo-watch)
[group('Maintenance')]
watch:
    cargo watch -x "test {{ pure_crates }} --target {{ host_target }}"

# --- Release --------------------------------------------------------------

# verify pure-logic crates package cleanly (no upload); driver crates require pure 0.X live on crates.io first — use release-dry-run-crate after Stage 1 publish
# dry-run packaging of the pure-logic crates
[group('Release')]
release-dry-run: verify
    cargo publish --dry-run -p bunting --target {{ host_target }}
    cargo publish --dry-run -p pennant --target {{ host_target }}
    cargo publish --dry-run -p ferriswheel --target {{ host_target }}

# verify one pure/AVR crate packages cleanly against host target (no upload). Use: just release-dry-run-crate rustyfarian-avr-ws2812
# dry-run packaging of one pure/AVR crate
[group('Release')]
release-dry-run-crate crate:
    cargo publish --dry-run -p {{ crate }} --target {{ host_target }}

# verify esp-hal driver packages cleanly against bare-metal target (no upload; requires riscv32imac-unknown-none-elf target installed)
# dry-run packaging of the esp-hal driver
[group('Release')]
release-dry-run-hal:
    cargo publish --dry-run -p rustyfarian-esp-hal-ws2812 --target {{ hal_target }}

# verify esp-idf driver packages cleanly against IDF target (no upload; requires espup)
# dry-run packaging of the esp-idf driver
[group('Release')]
release-dry-run-idf:
    cargo +esp publish --dry-run -p rustyfarian-esp-idf-ws2812 --target riscv32imac-esp-espidf --target-dir {{ idf_dir }} $(scripts/idf-build-dir.sh --config)

# publish one pure/AVR crate: just release-publish bunting
[confirm]
[group('Release')]
release-publish crate:
    cargo publish -p {{ crate }} --target {{ host_target }}

# publish esp-hal driver to crates.io (requires riscv32imac-unknown-none-elf target installed)
# publish the esp-hal driver to crates.io
[confirm]
[group('Release')]
release-publish-hal:
    cargo publish -p rustyfarian-esp-hal-ws2812 --target {{ hal_target }}

# publish the esp-idf driver to crates.io (requires espup)
[confirm]
[group('Release')]
release-publish-idf:
    cargo +esp publish -p rustyfarian-esp-idf-ws2812 --target riscv32imac-esp-espidf --target-dir {{ idf_dir }} $(scripts/idf-build-dir.sh --config)
