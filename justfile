# Rustyfarian WS2812 — development tasks
#
# The workspace defaults to the bare-metal ESP target (riscv32imac-unknown-none-elf)
# via .cargo/config.toml. IDF recipes explicitly pass --target {{ idf_target }}.
# Pure-crate recipes explicitly pass --target {{ host_target }}.
#
# `just --list` width guard: the name column is padded to 40, so keep each new recipe's short
# description to about 59 characters (a row under 100 columns). A few older rows still exceed
# that; do not add to them. Find the widest row with:
#   just --list | awk '{ print length }' | sort -rn | head -1

host_target := `scripts/host-target.sh`
pure_crates := "-p bunting -p ferriswheel -p pennant -p rustyfarian-avr-ws2812"
hal_target := "riscv32imac-unknown-none-elf"
idf_target := "riscv32imac-esp-espidf"
hal_crate := "-p rustyfarian-esp-hal-ws2812"
avr_nightly := "nightly-2025-04-27"

ramdisk := "/Volumes/RustBuilds"
# Detect a real mounted volume; `path_exists` would be fooled by a stale /Volumes/<name>
# directory that was never `hdiutil attach`-ed (or wasn't cleaned up after detach).
# scripts/ramdisk-mounted.sh is the byte-identical rustyfarian-ci copy.
ramdisk_mounted := shell(justfile_directory() + '/scripts/ramdisk-mounted.sh "' + ramdisk + '"')
hal_dir := if ramdisk_mounted == "true" { ramdisk + "/targets/hal/" + file_name(justfile_directory()) } else { "target/hal" }
idf_dir := if ramdisk_mounted == "true" { ramdisk + "/targets/idf/" + file_name(justfile_directory()) } else { "target/idf" }

# Guided bench runner: a local rustyfarian-bench checkout if RUSTYFARIAN_BENCH points at one,
# otherwise the pinned release from GitHub.
bench_dir := env("RUSTYFARIAN_BENCH", "")
bench := if bench_dir != "" { 'uv run --project "' + bench_dir + '" rustyfarian-bench' } else { "uvx --from git+https://github.com/datenkollektiv/rustyfarian-bench@v0.5.0 rustyfarian-bench" }

# act: a local rustyfarian-ci checkout if RUSTYFARIAN_CI points at one, otherwise the pushed @v1.
ci_dir := env("RUSTYFARIAN_CI", "")
ci_repo_flag := if ci_dir != "" { '--local-repository "datenkollektiv/rustyfarian-ci@v1=' + absolute_path(ci_dir) + '"' } else { "" }

# list available recipes (default)
_default:
    @just --list

# --- Build Environment ----------------------------------------------------

# show RAM disk status, resolved target dirs, sccache, and crates.io auth
[group('Build Environment')]
doctor:
    @scripts/doctor.sh "{{ ramdisk }}" "{{ hal_dir }}" "{{ idf_dir }}" "$(scripts/idf-build-dir.sh)" "$(scripts/idf-build-dir.sh --glob)"

# manage the RAM disk: attach | detach | prune [days]
[group('Build Environment')]
ramdisk action *args:
    @scripts/ramdisk.sh "{{ action }}" {{ args }}

# --- Build & Check --------------------------------------------------------

# build platform-independent crates
[group('Build & Check')]
build:
    cargo build {{ pure_crates }} --target {{ host_target }}

# build all crates including ESP-IDF (requires espup; does NOT cover rustyfarian-esp-hal-ws2812 or rustyfarian-avr-ws2812 — use check-hal / check-avr)
[doc('build every crate the ESP-IDF toolchain compiles')]
[group('Build & Check')]
build-all:
    cargo +esp build --workspace --exclude rustyfarian-esp-hal-ws2812 --exclude rustyfarian-avr-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }} $(scripts/idf-build-dir.sh --config)

# check platform-independent crates (no ESP toolchain)
[group('Build & Check')]
check:
    cargo check {{ pure_crates }} --target {{ host_target }}

# check all crates including ESP-IDF (requires espup; does NOT cover rustyfarian-esp-hal-ws2812 or rustyfarian-avr-ws2812 — use check-hal / check-avr)
[doc('check every crate the ESP-IDF toolchain compiles')]
[group('Build & Check')]
check-all:
    cargo +esp check --workspace --exclude rustyfarian-esp-hal-ws2812 --exclude rustyfarian-avr-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }} $(scripts/idf-build-dir.sh --config)

# check only the ESP-IDF driver crate (requires espup)
[group('Build & Check')]
check-idf:
    cargo +esp check -p rustyfarian-esp-idf-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }} $(scripts/idf-build-dir.sh --config)

# check the ESP-IDF driver on the IDE's default bare-metal target, as rust-analyzer sees it
[group('Build & Check')]
check-ide:
    cargo check -p rustyfarian-esp-idf-ws2812 --target {{ hal_target }} --target-dir {{ hal_dir }}

# c6 checks the library with the default features on riscv32imac. c3 runs a library pass, then an
# --examples pass with `rt` so hal_c3_pulse is compiled too (requires: just setup hal)
[doc('check the esp-hal driver on one RISC-V chip: c6 | c3')]
[group('Build & Check')]
check-hal chip="c6":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ chip }}" in
        c6)
            cargo check {{ hal_crate }} --target {{ hal_target }} --target-dir {{ hal_dir }}
            ;;
        c3)
            cargo check {{ hal_crate }} --target riscv32imc-unknown-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32c3,unstable,pennant
            cargo check {{ hal_crate }} --target riscv32imc-unknown-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32c3,unstable,pennant,rt --examples
            ;;
        *)
            echo "error: unknown chip '{{ chip }}' (expected c6 or c3)" >&2
            exit 1
            ;;
    esac

# check the esp-hal driver on the Xtensa ESP32 and ESP32-S3 targets (requires: just setup esp)
[doc('check the esp-hal driver on the Xtensa ESP32 and ESP32-S3 targets')]
[group('Build & Check')]
check-hal-xtensa:
    cargo +esp check {{ hal_crate }} --target xtensa-esp32-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32,unstable,pennant -Z build-std=core
    cargo +esp check {{ hal_crate }} --target xtensa-esp32-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32,unstable,pennant,rt --examples -Z build-std=core
    cargo +esp check {{ hal_crate }} --target xtensa-esp32s3-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32s3,unstable,pennant -Z build-std=core
    cargo +esp check {{ hal_crate }} --target xtensa-esp32s3-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32s3,unstable,pennant,rt --examples -Z build-std=core
    cargo +esp check {{ hal_crate }} --target xtensa-esp32-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32,unstable,pennant,async -Z build-std=core
    cargo +esp check {{ hal_crate }} --target xtensa-esp32s3-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32s3,unstable,pennant,async -Z build-std=core

# Async library pass on C6 and C3, then an --examples pass with `embassy,rt` so the *_async
# examples compile (hal_c6_*_async, hal_c3_pulse_async). `embassy` implies `async` and pulls esp-rtos.
# (requires: rustup target add riscv32imac-unknown-none-elf riscv32imc-unknown-none-elf)
[doc("check the esp-hal driver's async API on the ESP32-C6 and ESP32-C3 targets")]
[group('Build & Check')]
check-hal-async:
    cargo check {{ hal_crate }} --target {{ hal_target }} --target-dir {{ hal_dir }} --features async
    cargo check {{ hal_crate }} --target {{ hal_target }} --target-dir {{ hal_dir }} --features embassy,rt,esp-println,smart-leds --examples
    cargo check {{ hal_crate }} --target riscv32imc-unknown-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32c3,unstable,pennant,async
    cargo check {{ hal_crate }} --target riscv32imc-unknown-none-elf --target-dir {{ hal_dir }} --no-default-features --features esp32c3,unstable,pennant,rt,embassy,esp-println --examples

# Compare each crate's public API with a published release (default: the latest on crates.io),
# each on the target and features its check recipes use (requires: cargo install cargo-semver-checks)
[doc('semver-check every crate against crates.io: `just semver 0.8.0`')]
[group('Build & Check')]
semver baseline="":
    scripts/semver.sh "{{ baseline }}" "{{ host_target }}" "{{ hal_target }}"

# check the AVR SPI driver on host (no AVR toolchain)
[group('Build & Check')]
check-avr:
    cargo check -p rustyfarian-avr-ws2812 --target {{ host_target }}

# check the AVR driver on host with all features (bitbang, smart-leds-trait, pennant)
[doc('check the AVR driver on host with all features')]
[group('Build & Check')]
check-avr-all-features:
    cargo check -p rustyfarian-avr-ws2812 --features bitbang,smart-leds-trait,pennant --target {{ host_target }}

# check the AVR SPI driver against the real avr-none target with smart-leds-trait and pennant (requires: just setup avr, avr-gcc)
[doc('check AVR SPI driver on avr-none (smart-leds-trait+pennant)')]
[group('Build & Check')]
check-avr-target:
    RUSTFLAGS="-C target-cpu=atmega328p" cargo +{{ avr_nightly }} check -p rustyfarian-avr-ws2812 --features smart-leds-trait,pennant --target avr-none -Z build-std=core

# check the AVR bit-bang driver against the real avr-none target with all features (bitbang, smart-leds-trait, pennant) (requires: just setup avr, avr-gcc)
[doc('check the AVR bit-bang driver on avr-none, all features')]
[group('Build & Check')]
check-avr-target-bitbang:
    RUSTFLAGS="-C target-cpu=atmega328p" cargo +{{ avr_nightly }} check -p rustyfarian-avr-ws2812 --features bitbang,smart-leds-trait,pennant --target avr-none -Z build-std=core

# check ferriswheel with the smart-leds-trait feature — exercises the rgb-version-divergence guard; the smart-leds-compat alias is checked too
[doc("check ferriswheel's smart-leds-trait feature and its alias")]
[group('Build & Check')]
check-ferriswheel-smart-leds-trait:
    cargo check -p ferriswheel --features smart-leds-trait --target {{ host_target }}
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
[doc("test pennant's hal feature (SimpleLed adapter)")]
[group('Test & Lint')]
test-pennant-hal:
    cargo test -p pennant --features hal --target {{ host_target }}

# run clippy on pennant's hal feature — covers the SimpleLed / RgbGpioLed / RgbPwmLed adapters (off by default)
[doc("clippy pennant's hal feature (LED adapters)")]
[group('Test & Lint')]
clippy-pennant-hal:
    cargo clippy -p pennant --features hal --target {{ host_target }} -- -D warnings

# test the AVR driver on the host with all features (bitbang, smart-leds-trait, pennant)
[doc('test the AVR driver on the host with all features')]
[group('Test & Lint')]
test-avr-all-features:
    cargo test -p rustyfarian-avr-ws2812 --features bitbang,smart-leds-trait,pennant --target {{ host_target }}

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
[doc('clippy every crate the ESP-IDF toolchain lints')]
[group('Test & Lint')]
clippy-all:
    cargo +esp clippy --workspace --exclude rustyfarian-esp-hal-ws2812 --exclude rustyfarian-avr-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }} $(scripts/idf-build-dir.sh --config) -- -D warnings

# clippy the ESP-IDF driver crate only (requires espup)
[group('Test & Lint')]
clippy-idf:
    cargo +esp clippy -p rustyfarian-esp-idf-ws2812 --target {{ idf_target }} --target-dir {{ idf_dir }} $(scripts/idf-build-dir.sh --config) -- -D warnings

# run clippy on the esp-hal bare-metal driver (requires: rustup target add riscv32imac-unknown-none-elf)
[doc('clippy the esp-hal bare-metal driver')]
[group('Test & Lint')]
clippy-hal:
    cargo clippy {{ hal_crate }} --target {{ hal_target }} --target-dir {{ hal_dir }} -- -D warnings

# run clippy on the esp-hal driver's async API (requires: rustup target add riscv32imac-unknown-none-elf)
[doc('clippy the esp-hal driver async API')]
[group('Test & Lint')]
clippy-hal-async:
    cargo clippy {{ hal_crate }} --target {{ hal_target }} --target-dir {{ hal_dir }} --features async -- -D warnings

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
[doc('build the AVR Nano rainbow example (bit-bang)')]
[group('AVR Examples')]
build-avr-example:
    scripts/avr.sh "{{ avr_nightly }}" build

# build every binary in the AVR Nano example crate (default + bitbang_demo + spi_rainbow + bitbang_spike).
# --locked is the CI reproducibility gate: it fails loudly if Cargo.toml was edited without
# committing the regenerated Cargo.lock. Local dev recipes deliberately omit it.
[doc('build every binary in the AVR Nano example crate')]
[group('AVR Examples')]
build-avr-example-all-bins:
    cd examples/avr-nano-rainbow && cargo +{{ avr_nightly }} build --release -Z build-std=core --bins --locked

# build the AVR example against upstream avr-hal main instead of the pinned rev — the weekly
# early-warning check. Works on a throwaway copy in a temp dir, so no tracked file is touched
# even transiently; the `crates` symlink keeps the example's ../../crates path deps resolving.
[doc('build the AVR example against upstream avr-hal main')]
[group('AVR Examples')]
build-avr-example-upstream:
    scripts/build-avr-example-upstream.sh "{{ justfile_directory() }}" "{{ avr_nightly }}"

# build and flash the AVR Nano rainbow demo — bit-bang backend, recommended (requires: just setup avr, avr-gcc, ravedude)
[doc('flash the AVR Nano rainbow demo (bit-bang)')]
[group('AVR Examples')]
flash-avr-example:
    scripts/avr.sh "{{ avr_nightly }}" flash

# build and flash the production bit-bang PulseEffect demo (uses Ws2812BitBang from the driver crate)
[doc('flash the production bit-bang PulseEffect demo')]
[group('AVR Examples')]
flash-avr-bitbang-demo: (flash-avr-bin "bitbang_demo")

# build one AVR Nano binary by name (requires: just setup avr, avr-gcc)
[doc('build one AVR Nano binary by name')]
[group('AVR Examples')]
build-avr-bin bin:
    scripts/avr.sh "{{ avr_nightly }}" build "{{ bin }}"

# build and flash one AVR Nano binary by name; ravedude finds the board unless a port is given (requires: just setup avr, avr-gcc, ravedude)
[doc('build and flash one AVR Nano binary, optionally on a port')]
[group('AVR Examples')]
flash-avr-bin bin port="":
    scripts/avr.sh "{{ avr_nightly }}" flash "{{ bin }}" "{{ port }}"

# build and flash the SPI prerendered rainbow — DIAGNOSTIC ONLY; many strips render this as white-ish (see ADR 007)
[doc('flash the SPI prerendered rainbow (DIAGNOSTIC ONLY)')]
[group('AVR Examples')]
flash-avr-spi-rainbow: (flash-avr-bin "spi_rainbow")

# build and flash the AVR Nano bit-bang spike (frozen low-level reference, see docs/features/avr-bitbang-driver.md)
[doc('flash the AVR bit-bang spike (frozen reference)')]
[group('AVR Examples')]
flash-avr-bitbang-spike: (flash-avr-bin "bitbang_spike")

# --- ESP Examples ---------------------------------------------------------

# build a driver example; the crate is inferred from the hal_/idf_ prefix
[doc('build an example; crate inferred from the hal_/idf_ prefix')]
[group('ESP Examples')]
build-example example:
    scripts/build-example.sh "{{ example }}" "{{ hal_dir }}" "{{ idf_dir }}"

# ensure the IDF-built bootloader is in the build cache for the given chip
[doc('cache the IDF-built bootloader for a chip (c3|c6|esp32|s3)')]
[group('ESP Examples')]
ensure-bootloader chip:
    scripts/ensure-bootloader.sh "{{ chip }}" "{{ hal_dir }}" "{{ idf_dir }}"

# build and flash a driver example; the crate is inferred from the hal_/idf_ prefix
[doc('build and flash an example; crate inferred from prefix')]
[group('ESP Examples')]
run-example example:
    scripts/run-example.sh "{{ example }}" "{{ hal_dir }}" "{{ idf_dir }}"

# --- Flash & Monitor ------------------------------------------------------

# build and flash any example; crate auto-detected from name prefix (hal_* or idf_*)
[doc('build and flash any example (crate auto-detected)')]
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
[doc("erase the connected ESP device's flash completely")]
[confirm]
[group('Flash & Monitor')]
erase-flash:
    espflash erase-flash

# guided hardware check: `just bench s3`, `just bench c6 pulse`
[group('Flash & Monitor')]
bench target *args:
    {{ bench }} --repo . run "{{ target }}" {{ args }}

# list the bench scenarios
[group('Flash & Monitor')]
bench-list:
    {{ bench }} --repo . list

# probe the connected boards (chip and MAC per port)
[group('Flash & Monitor')]
bench-detect:
    {{ bench }} detect

# --- Documentation --------------------------------------------------------

# build rustdoc for platform-independent crates
[group('Documentation')]
doc:
    cargo doc {{ pure_crates }} --target {{ host_target }} --no-deps

# build rustdoc for the esp-hal driver, default features and with `async` (requires: rustup target add riscv32imac-unknown-none-elf)
[group('Documentation')]
doc-hal:
    cargo doc {{ hal_crate }} --target {{ hal_target }} --target-dir {{ hal_dir }} --no-deps
    cargo doc {{ hal_crate }} --target {{ hal_target }} --target-dir {{ hal_dir }} --no-deps --features async

# build and open docs in browser
[group('Documentation')]
doc-open:
    cargo doc {{ pure_crates }} --target {{ host_target }} --no-deps --open

# --- CI -------------------------------------------------------------------

# check code quality without modifying files
[group('CI')]
verify: fmt-check
    cargo check {{ pure_crates }} --target {{ host_target }}
    cargo clippy {{ pure_crates }} --target {{ host_target }} -- -D warnings
    cargo test {{ pure_crates }} --target {{ host_target }}
    cargo test -p pennant --features hal --target {{ host_target }}
    cargo clippy -p pennant --features hal --target {{ host_target }} -- -D warnings
    cargo check -p ferriswheel --features smart-leds-trait --target {{ host_target }}
    cargo check -p ferriswheel --features smart-leds-compat --target {{ host_target }}

# format, then verify (modifies files — local use only)
[group('CI')]
pre-commit: fmt verify

# CI-equivalent gate: verify, then deny
[group('CI')]
ci: verify deny

# CI job recipes: each GitHub Actions job runs exactly one of these (rustyfarian-ci conventions)
[doc('CI job: check formatting')]
[group('CI')]
ci-fmt: fmt-check

# CI job: lint the pure crates and pennant's hal feature
[group('CI')]
ci-clippy: clippy clippy-pennant-hal

# CI job: check and test the pure crates, incl. feature guards
[group('CI')]
ci-test: check test check-ferriswheel-smart-leds-trait test-pennant-hal

# CI job: cargo-deny licenses, advisories, bans and sources
[group('CI')]
ci-deny: deny

# CI job: cargo-audit advisories
[group('CI')]
ci-audit: audit

# CI job: check the esp-hal driver on the RISC-V targets
[group('CI')]
ci-riscv: && check-hal (check-hal "c3") check-hal-async
    rustup target add riscv32imac-unknown-none-elf riscv32imc-unknown-none-elf

# CI job: check the esp-hal driver on Xtensa (requires espup)
[group('CI')]
ci-xtensa: check-hal-xtensa

# CI job: check the AVR driver and link the AVR example bins
[group('CI')]
ci-avr: check-avr-target check-avr-target-bitbang build-avr-example-all-bins

# CI job: build the AVR example against upstream avr-hal main
[group('CI')]
ci-avr-upstream: build-avr-example-upstream

# CI job: link every ESP-IDF example of one chip (c3|c6|esp32|s3)
[group('CI')]
ci-idf chip:
    scripts/ci-idf.sh "{{ chip }}" "{{ hal_dir }}" "{{ idf_dir }}"

# Set RUSTYFARIAN_CI to a local rustyfarian-ci checkout to test unpushed reusable workflows.
# The default runs the host jobs; act ignores `paths:` filters, so run the toolchain-heavy
# cross-target jobs explicitly (`just act riscv`; requires Docker + act).
[doc('run CI jobs via act: all host jobs, or one (`just act fmt`)')]
[group('CI')]
act job="":
    #!/usr/bin/env bash
    set -euo pipefail
    for j in {{ if job == "" { "fmt clippy test deny audit" } else { job } }}; do
        act -j "$j" --pull=false -P ubuntu-latest=catthehacker/ubuntu:act-latest {{ ci_repo_flag }}
    done

# --- Setup ----------------------------------------------------------------

# install components: just setup [tools|hal|avr|esp|all]
[group('Setup')]
setup component="all":
    scripts/setup.sh "{{ component }}" "{{ avr_nightly }}"

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
[doc('clean ESP-IDF artifacts and esp-idf-sys hash dirs')]
[group('Maintenance')]
clean-idf:
    cargo clean -p rustyfarian-esp-idf-ws2812 --target-dir "{{ idf_dir }}"
    @scripts/idf-cache.sh clean "{{ idf_dir }}"

# drop superseded esp-idf-sys build dirs, keeping the newest per target (fixes "multiple IDF-built bootloaders")
[doc('drop superseded esp-idf-sys dirs, keep newest per target')]
[group('Maintenance')]
clean-idf-stale:
    @scripts/idf-cache.sh clean-stale

# remove the persistent IDF build-dir cache entirely (the relocated esp-idf-sys CMake trees)
[doc('remove the persistent IDF build-dir cache entirely')]
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
[doc('dry-run packaging of the pure-logic crates')]
[group('Release')]
release-dry-run: verify
    cargo publish --dry-run -p bunting --target {{ host_target }}
    cargo publish --dry-run -p pennant --target {{ host_target }}
    cargo publish --dry-run -p ferriswheel --target {{ host_target }}

# verify one pure/AVR crate packages cleanly against host target (no upload). Use: just release-dry-run-crate rustyfarian-avr-ws2812
[doc('dry-run packaging of one pure/AVR crate')]
[group('Release')]
release-dry-run-crate crate:
    cargo publish --dry-run -p {{ crate }} --target {{ host_target }}

# verify esp-hal driver packages cleanly against bare-metal target (no upload; requires riscv32imac-unknown-none-elf target installed)
[doc('dry-run packaging of the esp-hal driver')]
[group('Release')]
release-dry-run-hal:
    cargo publish --dry-run -p rustyfarian-esp-hal-ws2812 --target {{ hal_target }}

# verify esp-idf driver packages cleanly against IDF target (no upload; requires espup)
[doc('dry-run packaging of the esp-idf driver')]
[group('Release')]
release-dry-run-idf:
    cargo +esp publish --dry-run -p rustyfarian-esp-idf-ws2812 --target riscv32imac-esp-espidf --target-dir {{ idf_dir }} $(scripts/idf-build-dir.sh --config)

# publish one pure/AVR crate: just release-publish bunting
[confirm]
[group('Release')]
release-publish crate:
    cargo publish -p {{ crate }} --target {{ host_target }}

# publish esp-hal driver to crates.io (requires riscv32imac-unknown-none-elf target installed)
[doc('publish the esp-hal driver to crates.io')]
[confirm]
[group('Release')]
release-publish-hal:
    cargo publish -p rustyfarian-esp-hal-ws2812 --target {{ hal_target }}

# publish the esp-idf driver to crates.io (requires espup)
[confirm]
[group('Release')]
release-publish-idf:
    cargo +esp publish -p rustyfarian-esp-idf-ws2812 --target riscv32imac-esp-espidf --target-dir {{ idf_dir }} $(scripts/idf-build-dir.sh --config)

import? '.devcontainer/yolo.just'
