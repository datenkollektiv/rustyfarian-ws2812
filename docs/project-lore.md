# Project Lore

This file records non-obvious technical discoveries: facts that caused surprising
failures, took significant time to debug, or would save a future developer 30+
minutes if known upfront.

Refer to `CLAUDE.md` and the `/project-lore` skill for recording guidelines.

---

## Build System

**The crate that produces the final binary or example must call `embuild::espidf::sysenv::output()` in its own `build.rs`.**
Wrapping `esp-idf-sys` in a library crate (e.g. `rustyfarian-esp-idf-ws2812`) is not sufficient: the `--ldproxy-linker` link arg emitted by `esp-idf-sys`'s build script is not reliably forwarded to the final link step, causing `ldproxy: Cannot locate argument '--ldproxy-linker'`.
Fix: add a `build.rs` to every crate that owns binaries or examples, calling `embuild::espidf::sysenv::output()`, and add `embuild` to its `[build-dependencies]`.

**Deleting the cached `esp-idf-sys` build output is required when `sdkconfig.defaults` is first introduced.**
Cargo's `rerun-if-changed` does not fire for a file that didn't exist before, so the IDF build script is not re-run on the first commit of `sdkconfig.defaults`.
Fix: remove `target/<triple>/debug/build/esp-idf-sys-<hash>/` and rebuild. Subsequent edits to the file retrigger correctly.

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

## esp-hal Bare-Metal Driver

**`esp-hal` bare-metal binaries must embed an IDF-compatible app descriptor or the IDF v5.3.3 bootloader rejects them with `boot_comm: Image requires efuse blk rev >= vX.Y, but chip is v0.3`.**
The bootloader reads `min_efuse_blk_rev_full` from the `esp_app_desc_t` offset; without a valid descriptor it interprets garbage bytes as a minimum efuse revision the chip can't meet. `--ignore-app-descriptor` only bypasses espflash's check, not the on-chip one. Affects both debug and release.
Fix: depend on the workspace-pinned `esp-bootloader-esp-idf` with `default-features = false`, forward the chip feature, and invoke `esp_bootloader_esp_idf::esp_app_desc!();` at module level in each bare-metal example.

**`esp-hal` bare-metal examples must be built with `--release`; debug builds risk malfunctioning on timing-sensitive peripherals.**
esp-hal emits a compile-time warning when the dev profile is used. WS2812 RMT examples may *appear* to work in debug because RMT timing is not CPU-bound, but the warning is real for other peripherals.
Fix: always pass `--release`; the binary is at `target/<triple>/release/examples/<name>`.

**`esp-hal 1.1.0` split the RMT TX builder: `configure_tx(pin, config)` is gone — the new pattern is `configure_tx(&config).unwrap().with_pin(pin)`.**
The pin moved from a `configure_tx` parameter to a chained `.with_pin(...)` call so that channel configuration can be reused independently of pin assignment.
The migration is mechanical for our examples but invasive — every `hal_*` example uses this pattern.
Compile-time error in 1.1.0 with the old call site: `error[E0061]: this method takes 1 argument but 2 arguments were supplied … unexpected argument #1 of type 'GPIO18<'static>'`.

**`embassy-executor 0.10.0` reshaped the task-spawn API: `Spawner::spawn` now returns `()`, and `#[embassy_executor::task]` functions return `Result<SpawnToken, SpawnError>`.**
The old idiom `spawner.spawn(task()).unwrap()` no longer compiles; the unwrap moves *inside* the spawn argument: `spawner.spawn(task().unwrap())`.
`Spawner::must_spawn` does not exist on `embassy-executor 0.10.0` despite the compiler suggesting "method `spawn` with a similar name" — the explicit pattern is the supported one.
Prefer `spawner.spawn(task().expect("<task name> spawn token"))` over a bare `unwrap()` on embedded targets; named messages survive into release-mode panic prints and make field debugging tractable.

**`embassy-sync 0.8.0` made `NoopRawMutex` `!Sync`, so `Signal<NoopRawMutex, _>` / `Mutex<NoopRawMutex, _>` in a `static` now fail with `*mut () cannot be shared between threads safely`.**
Intentional upstream: `NoopRawMutex` ("single executor, no contention") is fundamentally incompatible with global statics, which the language treats as `Sync`. `ThreadModeRawMutex` would satisfy `Sync` but is `cfg(cortex_m)`-only, unavailable on RISC-V.
Fix: switch the static to `CriticalSectionRawMutex`. For non-static, executor-local primitives passed by reference, `NoopRawMutex` remains the zero-cost choice.

---

## espflash

**`espflash 4.x --ignore-app-descriptor` only bypasses the "descriptor missing" check (bare-metal binaries), not the "descriptor misaligned" check.**
A binary built for the wrong MCU (wrong `memory.ld`) places the app descriptor at a misaligned flash offset; `--ignore-app-descriptor` does not suppress that error.
The real fix is to use the correct target and MCU — the flag is not a substitute for correct build configuration.

**`espflash 4.3.0` bundles an ESP-IDF `release/v5.5` branch bootloader, which breaks IDF and esp-hal binaries built against v5.3.3 (64 KB vs 32 KB MMU pages).**
IDF symptom: `Segment 0 load address …, doesn't match data …` (page-offset mismatch). HAL symptom: `Failed to fetch app description header!` (IDF `release/v5.5` branch is stricter about `esp_app_desc_t` placement). Both stem from the wrong bootloader being flashed.
Fix: pass `--bootloader target/<idf-triple>/debug/build/esp-idf-sys-<hash>/out/build/bootloader/bootloader.bin` (v5.3.3 — works for both binary types). The `run-example` justfile recipe handles it; `ensure-bootloader` builds the IDF cache on demand. Re-verify when upgrading either espflash or ESP-IDF.

**`esp-idf-sys`'s build script hard-exits with "Unsupported target 'riscv32imac-unknown-none-elf'" even when that target is only the workspace IDE/analysis default and the IDF crate is never explicitly requested.**
`esp-idf-sys` is an unconditional transitive dependency of `rustyfarian-esp-idf-ws2812`; Cargo always pulls it into the dep graph and runs its build script for every target, including the bare-metal default.
Fix requires three coordinated changes: (1) move `esp-idf-hal` (and `anyhow`, which requires `std`) to `[target.'cfg(target_os = "espidf")'.dependencies]` in `Cargo.toml` — this removes `esp-idf-sys` from the bare-metal dep graph entirely; (2) add `#![cfg(target_os = "espidf")]` to `src/lib.rs` so the crate compiles as an empty library on non-IDF targets; (3) guard `embuild::espidf::sysenv::output()` in `build.rs` with `if CARGO_CFG_TARGET_OS == "espidf"` — the build script still runs (build scripts always do) but exits early.

**`cargo clean --manifest-path <pkg>/Cargo.toml` silently reports "Removed 0 files" when the package was built with a pinned nightly toolchain and the command is invoked from a workspace root that uses stable.**
rustup resolves the toolchain from the *calling* directory, not from the manifest path.
Running from the workspace root (no `rust-toolchain.toml`) picks stable; stable cargo does not recognise artefacts built by a nightly toolchain and skips them.
Affected: `examples/avr-nano-rainbow/` (pins `nightly-2025-04-27` in its own `rust-toolchain.toml`).
Fix: `rm -rf examples/avr-nano-rainbow/target` — toolchain-independent, always clears the directory.

**Any edit to `Cargo.toml` (a new `[[example]]` entry, or a dependency version bump) can cause Cargo to assign a new hash to the `esp-idf-sys` build directory, leaving two bootloader artifacts that the flash script refuses to resolve silently.**
`ensure-bootloader.sh` intentionally errors with `multiple IDF-built bootloaders found` rather than picking one arbitrarily, because choosing the wrong bootloader produces a silent boot-loop.

**`cargo clean -p esp-idf-sys` no longer fixes this** — it reports "Removed 0 files". Since the `build.build-dir` split, the artefacts live under `~/Library/Caches/rustyfarian-cargo-build/<workspace-hash>/`, not under `target/`, and a bare `cargo clean -p` only searches the default target-dir. The old advice predates the split and is a dead end that looks like a no-op success.

Fix: `just clean-idf-stale` — drops superseded dirs and keeps the newest per target. `just clean-idf-cache` resets everything instead, at the cost of a full IDF rebuild across all architectures.

**It recurs once per architecture, which reads like "multiple ESP32 targets stopped working".** A dependency bump rehashes `esp-idf-sys` for *every* IDF target at once, but each target only grows its second directory the next time it is built. Cleaning the target you happen to be testing therefore leaves the others armed: on 2026-08-12 the `embuild 0.33.1 → 0.33.3` bump broke `riscv32imac` (C6) immediately, then `riscv32imc` (C3) hours later on first C3 flash, with `xtensa-esp32` still pending. The targets do not conflict with each other — the bootloader glob is scoped per `$idf_target`; it is one cause surfacing lazily.

Diagnostic signal: two `esp-idf-sys-<hash>` directories under the *same* target, similar size (~150–185 MB each), mtimes straddling the `Cargo.toml` edit that caused the rehash.

---

## Example GPIO Pins

**C3 and C6 examples use different WS2812 data pins (C3 = GPIO4, C6 = GPIO18); copying the C3 pin into a C6 binary fails silently with no LED output.**
The app boots, RMT transmits successfully, `set_pixels_slice` returns `Ok` — the signal just goes to the wrong pin and the LEDs hold their last colour.
Fix: `peripherals.GPIO4` for C3, `peripherals.GPIO18` for C6.

---

## ESP-IDF Runtime

**`esp-idf-hal 0.46.2` `send_and_wait` panics in ISR context: `EncoderWrapper::From<rmt_encode_state_t>` only matches single values (`0x00`, `0x01`, `0x02`) and hits the `_` arm on bitwise-OR'd flags like `COMPLETE | MEM_FULL = 0x03` that the C encoder legitimately returns.**
`send_and_wait` wraps even C-native encoders (`BytesEncoder`) in a Rust `EncoderWrapper`, inserting a Rust callback in the ISR path. The panic handler then calls `usb_serial_jtag_write` → `_lock_acquire_recursive`, which aborts (recursive mutexes are illegal in ISRs).
Symptom: `abort() was called at PC 0x...` with trace through `lock_acquire_generic` → `usb_serial_jtag_write`.
Fix: use `start_send` + `wait_all_done` directly with `BytesEncoder` (a `RawEncoder`), bypassing `EncoderWrapper` so the ISR calls the C encode function with no Rust wrapper.

**The new RMT API (`TxChannelDriver`) registers ISR callbacks via `rmt_tx_register_event_callbacks`, requiring a larger FreeRTOS ISR stack than the legacy API.**
The default `CONFIG_FREERTOS_ISR_STACKSIZE=1536` overflows with the new API's callback overhead.
Symptom: `Guru Meditation Error: Stack protection fault` immediately after `WS2812RMT::new()` succeeds.
Fix: add `CONFIG_FREERTOS_ISR_STACKSIZE=4096` to `sdkconfig.defaults` and clean the IDF build cache.

**The default ESP-IDF main task stack (3584 bytes) overflows in debug builds when `set_pixels_slice` loops over 12+ WS2812 LEDs.**
`color_to_pulses` in `rustyfarian-esp-idf-ws2812` returns `[Pulse; 48]` (~192 bytes) per LED on the stack.
Debug builds retain larger temporaries and do not optimise stack usage, so 12 LEDs consumes ~2300+ bytes just for pulse arrays before accounting for other frames.
The symptom is a `Guru Meditation Error: Core 0 panic'ed (Stack protection fault)` immediately after `WS2812RMT init OK`.
Fix: add `sdkconfig.defaults` at the workspace root with `CONFIG_ESP_MAIN_TASK_STACK_SIZE=8192`.

---

## Scripts & Shell

**`set -eo pipefail` causes silent script exit when `ls` targets an unmatched glob inside a command substitution.**
`ls glob-pattern 2>/dev/null | head -1` exits non-zero when the glob matches nothing; with `pipefail`, the pipeline's non-zero exit propagates through `var=$(...)`, and bash exits the script before any `if [ -z "$var" ]` guard runs.
The failure is invisible: the script exits with code 1 and no output, so the parent script and `just` recipe fail with only a line number.
Fix: append `|| true` to the pipeline: `var=$(ls glob 2>/dev/null | head -1 || true)`.

---

## Rust Syntax Gotchas

**`rustfmt` refuses to parse a file that contains `(expr) as u8 < rhs` — it misreads `u8 < rhs` as the start of a generic argument list.**
The error is: `` `<` is interpreted as a start of generic arguments for `u8`, not a comparison ``.
This causes `cargo fmt` to exit non-zero and refuse to format any file in the crate, blocking the whole `pre-commit` pipeline.
Fix: add a second pair of parentheses: `((expr) as u8) < rhs`; the inner cast is unambiguous with its own grouping.

---

## Local CI with `act`

**`rustsec/audit-check@v2` requires a real `GITHUB_TOKEN` for GitHub API calls; it fails with any dummy token and cannot be used with `act` in typical local setups.**
The action posts PR annotations via the GitHub API, which requires a token with `checks: write` permissions.
Passing `-s GITHUB_TOKEN=dummy` results in an authentication error during the action's API calls.
Fix: replace the action step with two plain steps — `cargo install cargo-audit --locked` and `cargo audit`.
This produces the same vulnerability report output, works locally with no token, and is simpler to read in CI logs.

**`act -s GITHUB_TOKEN=<any-value>` forwards the value as HTTP Basic auth when cloning third-party actions, breaking the download with `authentication required: Invalid username or token`.**
`act` treats `GITHUB_TOKEN` as a credential for all GitHub HTTP operations, not only steps that explicitly use `${{ secrets.GITHUB_TOKEN }}`.
Fix: omit `-s GITHUB_TOKEN=…` entirely when the workflow doesn't need it. If a real token is needed for a specific step, scope it via a step-level `env:` block instead of a global `act` secret.

---

## Developer Tooling

**`.claude/hooks/just-enforcer.sh` blocks Bash commands whose first word matches a binary wrapped by any `justfile` recipe but allows tools no recipe wraps (e.g. `sed`, `perl`, `find`, `git`).**
Batch text edits via `sed -i ''` and `perl -i -0pe` therefore remain available even though direct `cargo` is funnelled through `just`.
Side effect during settings cleanup: any `Bash(cargo *)` permission entries are functionally **dead** while the enforcer is active — they look granted but the hook intercepts before the permission resolver runs.

**RustRover does not use rust-analyzer as its backend and silently ignores `rust-analyzer.toml`.**
RustRover ships its own proprietary Kotlin-based analysis engine; the `[cargo] target` key in `rust-analyzer.toml` has no effect.
RustRover also has open bugs (RUST-12562, RUST-17656) where it fails to read the default target from `.cargo/config.toml` automatically.
Fix: set the target triple manually via the resolve-context switcher in the status bar (bottom-right); source `~/export-esp.sh` before launching RustRover so `LIBCLANG_PATH` is available at IDE startup.
The committed `rust-analyzer.toml` is still correct — it benefits all rust-analyzer-backed editors (VS Code, Neovim, Helix, Zed) and is harmless to RustRover.

**Claude Code Stop hooks only support `type: "command"`; `type: "prompt"` is not a valid hook type and produces "Stop hook error: JSON validation failed".**
There is no built-in AI-evaluation hook type; the schema rejects unrecognised `type` values immediately.
Fix: use `type: "command"` pointing at a shell script, and call `claude -p` inside the script to preserve AI-based evaluation if required.

**The Stop hook `decision` field accepts `"approve" | "block"`, not `"allow"`.**
Outputting `{"decision": "allow"}` fails schema validation; the correct value to permit the session to end is `"approve"`.
Fix: use `{"decision": "approve"}` or simply exit 0 with no output to allow the stop.

---

## AVR WS2812 Driver: SPI Prerendered Encoding Limitation

**`rustyfarian-avr-ws2812` SPI-prerendered output can latch as stable white-ish (proportional brightness but indistinguishable channels) on the *same physical strip* that the ESP-IDF and esp-hal drivers handle correctly.**
The 2 MHz SPI / 4-bits-per-WS2812-bit encoding emits `T0H = 0.5 µs` (right at the chip's 0/1 decision threshold) and `T1H = 1.5 µs` (well outside the WS2812B nominal `T1H ≈ 0.7 µs`); correctness then depends on per-chip tolerance which varies between WS2812 / WS2812B / clone variants. ESP RMT synthesises native timing (`T0H ≈ 400 ns`, `T1H ≈ 700 ns`) and does not hit this.
Smoking-gun diagnostic: with `NUM_LEDS = 1`, downstream LEDs in the chain still flicker — proof that LED 1 is not consuming exactly 24 bits, so partial/extra bits leak via the pass-through. This confirms encoding-level bit-counting unreliability as the cause and rules out colour-only theories (GRB order, PulseEffect math, brightness scaling). Crystal mismatch, power supply, cable length, and clone-vs-genuine board were also ruled out (2026-05-04).
**Lesson: a working ESP setup is *not* sufficient validation for the AVR driver — AVR hardware against the target strip type is required.** Resolution (cycle-counted inline-`asm!` bit-bang as the recommended backend) is documented in [`docs/adr/007-avr-ws2812-driver-strategy.md`](adr/007-avr-ws2812-driver-strategy.md).
