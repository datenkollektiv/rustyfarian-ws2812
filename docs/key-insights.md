# Key Insights

This file records non-obvious technical discoveries: facts that caused surprising
failures, took significant time to debug, or would save a future developer 30+
minutes if known upfront.

Refer to `CLAUDE.md` and the `/key-insights` skill for recording guidelines.

---

## Build System

**The crate that produces the final binary or example must call `embuild::espidf::sysenv::output()` in its own `build.rs`.**
Wrapping `esp-idf-sys` in a library crate (e.g. `rustyfarian-esp-idf-ws2812`) is not sufficient: the `--ldproxy-linker` link arg emitted by `esp-idf-sys`'s build script is not reliably forwarded to the final link step, causing `ldproxy: Cannot locate argument '--ldproxy-linker'`.
Fix: add a `build.rs` to every crate that owns binaries or examples, calling `embuild::espidf::sysenv::output()`, and add `embuild` to its `[build-dependencies]`.

**Deleting the cached `esp-idf-sys` build output is required for `sdkconfig.defaults` changes to take effect.**
Cargo's incremental build does not re-run `esp-idf-sys`'s build script when a new `sdkconfig.defaults` file is created (the file didn't exist before, so no `rerun-if-changed` fires).
Fix: remove `target/<triple>/debug/build/esp-idf-sys-<hash>/` and rebuild; alternatively `cargo clean` (slower).
Note: this special case applies only when the file is introduced for the first time; once it exists, subsequent edits may retrigger the build script correctly.

---

## Toolchain & Dependencies

**The `esp` toolchain (installed by `espup`) is required to compile `*-esp-espidf` targets; the stable toolchain does not know these targets.**
`cargo build --target riscv32imc-esp-espidf` with the stable toolchain produces `error[E0463]: can't find crate for core`.
Fix: prefix with `cargo +esp build …` or add `channel = "esp"` to a `rust-toolchain.toml`.

**`EffectError` from `ferriswheel` is `no_std` and does not implement `std::error::Error`, so the `?` operator cannot convert it to `anyhow::Error` in std contexts.**
Using `RainbowEffect::new(N)?` in an `anyhow::Result`-returning `fn main` fails at compile time with a trait bound error on `std::error::Error`.
Fix: use `.unwrap()` (or `.expect()`) for infallible construction validated at build time, matching the bare-metal pattern.

---

## ESP32 Target Selection

**ESP32-C3 requires the `riscv32imc-esp-espidf` target (no atomics, RV32IMC), not `riscv32imac-esp-espidf` (RV32IMAC, for C6/H2).**
Building C3 with the C6 target may compile far enough to produce an image, but the result is invalid: `memory.ld` is wrong for the chip, placing the app descriptor at the wrong flash offset.
The ESP-IDF toolchain also rejects the mismatch explicitly: `esp32c3 is not amongst MCUs supported by target 'riscv32imac-esp-espidf'`.
Fix: map `c3` → `riscv32imc-esp-espidf` and `c6` → `riscv32imac-esp-espidf` in build recipes; see `justfile` `idf)` case.

**`MCU="esp32c6"` in `.cargo/config.toml [env]` applies to all IDF builds; a shell-level env var must override it per-example.**
`[env]` entries are only applied when the variable is not already set in the shell.
Fix: set `MCU="esp32${chip_variant}"` in the justfile recipe as a shell-level prefix, which takes precedence over `[env]`.

---

## esp-hal + IDF Bootloader: App Descriptor Required

**`esp-hal` bare-metal binaries must embed an IDF-compatible app descriptor or the IDF v5.3.3 bootloader will reject them with `boot_comm: Image requires efuse blk rev >= vX.Y, but chip is v0.3`.**
The bootloader's `boot_comm` module reads `min_efuse_blk_rev_full` from the offset where `esp_app_desc_t` is expected.
Without a valid descriptor, it reads garbage bytes from the bare-metal binary and interprets them as a minimum efuse block revision that the chip (e.g. v0.3) cannot meet.
The `--ignore-app-descriptor` espflash flag only suppresses espflash's own descriptor check; it does not bypass the on-chip bootloader check.
Fix: add `esp-bootloader-esp-idf = { version = "0.4.0", default-features = false }` to workspace dependencies,
forward the chip feature (`esp32c6 = ["esp-hal/esp32c6", "esp-bootloader-esp-idf/esp32c6"]`),
and invoke `esp_bootloader_esp_idf::esp_app_desc!();` at module level in each bare-metal example.
Note: this applies to both `--release` and debug builds — `--release` only changes which garbage bytes appear at the descriptor offset.

---

## esp-hal Build Profile

**`esp-hal` bare-metal examples must be built with `--release`; debug builds may malfunction on timing-sensitive peripherals.**
esp-hal emits a compile-time warning if the dev profile is used: "The dev profile can potentially be one or more orders of magnitude slower than release, and may cause issues with timing-sensitive peripherals and/or devices."
For WS2812 RMT examples the LED output may appear to work in debug mode (hardware RMT timing is not CPU-bound), but the binary is larger, slower, and the warning indicates real risk for other peripherals.
Fix: always pass `--release` to `cargo build` for `hal_*` examples; the output binary is at `target/<triple>/release/examples/<name>`, not `debug/`.

---

## espflash

**`espflash 4.x --ignore-app-descriptor` only bypasses the "descriptor missing" check (bare-metal binaries), not the "descriptor misaligned" check.**
A binary built for the wrong MCU (wrong `memory.ld`) places the app descriptor at a misaligned flash offset; `--ignore-app-descriptor` does not suppress that error.
The real fix is to use the correct target and MCU — the flag is not a substitute for correct build configuration.

**`espflash 4.3.0` bundles an ESP-IDF v5.5.1 bootloader; this breaks both IDF and esp-hal bare-metal examples when the build targets ESP-IDF v5.3.3.**
The v5.5.1 bootloader uses 64 KB MMU pages; v5.3.3 builds use 32 KB pages.
IDF symptom: `Segment 0 load address 0x42118020, doesn't match data 0x00010020` — the page-offset check fails.
HAL (bare-metal) symptom: `Failed to fetch app description header!` — v5.5.1 is stricter about the `esp_app_desc_t` location/format.
Both failures share the same root cause: the wrong bootloader is being flashed.
Fix: pass `--bootloader target/<idf-triple>/debug/build/esp-idf-sys-<hash>/out/build/bootloader/bootloader.bin` to espflash.
The v5.3.3 bootloader (from any IDF example build artifact) works for both IDF and esp-hal binaries.
The `run-example` justfile recipe handles this automatically; `ensure-bootloader chip` builds the IDF cache on demand if missing.
Note: this workaround is version-specific (espflash 4.3.0, ESP-IDF v5.3.3 vs v5.5.1); re-verify if upgrading either.

---

## Example GPIO Pins

**C3 and C6 examples use different GPIO pins for the WS2812 data line; copying from the C3 example to a C6 example will produce a binary that runs silently with no LED output.**
The ESP32-C3 examples wire the WS2812 ring to GPIO4.
The ESP32-C6 examples wire the WS2812 ring to GPIO18.
The mismatch is silent: the app boots, RMT transmits successfully, and no error is reported — the signal simply goes to the wrong pin.
Symptom: full successful boot log, `set_pixels_slice` returns `Ok`, LEDs hold their last color and never change.
Fix: use `peripherals.GPIO4` for C3 examples and `peripherals.GPIO18` for C6 examples.

---

## ESP-IDF Runtime

**The default ESP-IDF main task stack (3584 bytes) overflows in debug builds when `set_pixels_slice` loops over 12+ WS2812 LEDs.**
`color_to_pulses` in `rustyfarian-esp-idf-ws2812` returns `[Pulse; 48]` (~192 bytes) per LED on the stack.
Debug builds retain larger temporaries and do not optimise stack usage, so 12 LEDs consumes ~2300+ bytes just for pulse arrays before accounting for other frames.
The symptom is a `Guru Meditation Error: Core 0 panic'ed (Stack protection fault)` immediately after `WS2812RMT init OK`.
Fix: add `sdkconfig.defaults` at the workspace root with `CONFIG_ESP_MAIN_TASK_STACK_SIZE=8192`.

---

## Scripts & Shell

**`set -eo pipefail` causes silent script exit when `ls` targets an unmatched glob inside a command substitution.**
`ls glob-pattern 2>/dev/null | head -1` exits non-zero when the glob matches nothing; with `pipefail`, the pipeline's non-zero exit propagates through `var=$(...)`, and bash exits the script before the `if [ -z "$var" ]` guard runs.
The failure is invisible: the script exits with code 1 and no output, so the parent script and `just` recipe fail with only a line number.
This was the root cause of `just run hal_c3_pulse` failing on line 65 — `ensure-bootloader.sh` was silently aborting at the bootloader-cache lookup, never reaching the IDF build that populates the cache.
Fix: append `|| true` to the pipeline: `var=$(ls glob 2>/dev/null | head -1 || true)`.

---

## Developer Tooling

**Claude Code Stop hooks only support `type: "command"`; `type: "prompt"` is not a valid hook type and produces "Stop hook error: JSON validation failed".**
There is no built-in AI-evaluation hook type; the schema rejects unrecognised `type` values immediately.
Fix: use `type: "command"` pointing at a shell script, and call `claude -p` inside the script to preserve AI-based evaluation if required.

**The Stop hook `decision` field accepts `"approve" | "block"`, not `"allow"`.**
Outputting `{"decision": "allow"}` fails schema validation; the correct value to permit the session to end is `"approve"`.
Fix: use `{"decision": "approve"}` or simply exit 0 with no output to allow the stop.
