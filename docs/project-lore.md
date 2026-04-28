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

## esp-hal Bare-Metal Driver

**`esp-hal` bare-metal binaries must embed an IDF-compatible app descriptor or the IDF v5.3.3 bootloader will reject them with `boot_comm: Image requires efuse blk rev >= vX.Y, but chip is v0.3`.**
The bootloader's `boot_comm` module reads `min_efuse_blk_rev_full` from the offset where `esp_app_desc_t` is expected.
Without a valid descriptor, it reads garbage bytes from the bare-metal binary and interprets them as a minimum efuse block revision that the chip (e.g. v0.3) cannot meet.
The `--ignore-app-descriptor` espflash flag only suppresses espflash's own descriptor check; it does not bypass the on-chip bootloader check.
Fix: add `esp-bootloader-esp-idf = { version = "0.4.0", default-features = false }` to workspace dependencies,
forward the chip feature (`esp32c6 = ["esp-hal/esp32c6", "esp-bootloader-esp-idf/esp32c6"]`),
and invoke `esp_bootloader_esp_idf::esp_app_desc!();` at module level in each bare-metal example.
Note: this applies to both `--release` and debug builds — `--release` only changes which garbage bytes appear at the descriptor offset.

**`esp-hal` bare-metal examples must be built with `--release`; debug builds may malfunction on timing-sensitive peripherals.**
esp-hal emits a compile-time warning if the dev profile is used: "The dev profile can potentially be one or more orders of magnitude slower than release, and may cause issues with timing-sensitive peripherals and/or devices."
For WS2812 RMT examples the LED output may appear to work in debug mode (hardware RMT timing is not CPU-bound), but the binary is larger, slower, and the warning indicates real risk for other peripherals.
Fix: always pass `--release` to `cargo build` for `hal_*` examples; the output binary is at `target/<triple>/release/examples/<name>`, not `debug/`.

**`esp-hal 1.1.0` split the RMT TX builder: `configure_tx(pin, config)` is gone — the new pattern is `configure_tx(&config).unwrap().with_pin(pin)`.**
The pin moved from a `configure_tx` parameter to a chained `.with_pin(...)` call so that channel configuration can be reused independently of pin assignment.
The migration is mechanical for our examples but invasive — every `hal_*` example uses this pattern.
Compile-time error in 1.1.0 with the old call site: `error[E0061]: this method takes 1 argument but 2 arguments were supplied … unexpected argument #1 of type 'GPIO18<'static>'`.

**`embassy-executor 0.10.0` reshaped the task-spawn API: `Spawner::spawn` now returns `()`, and `#[embassy_executor::task]` functions return `Result<SpawnToken, SpawnError>`.**
The old idiom `spawner.spawn(task()).unwrap()` no longer compiles; the unwrap moves *inside* the spawn argument: `spawner.spawn(task().unwrap())`.
`Spawner::must_spawn` does not exist on `embassy-executor 0.10.0` despite the compiler suggesting "method `spawn` with a similar name" — the explicit pattern is the supported one.
Prefer `spawner.spawn(task().expect("<task name> spawn token"))` over a bare `unwrap()` on embedded targets; named messages survive into release-mode panic prints and make field debugging tractable.

**`embassy-sync 0.8.0` made `NoopRawMutex` `!Sync` (carries `PhantomData<*mut ()>`), so any `Signal<NoopRawMutex, _>` or `Mutex<NoopRawMutex, _>` placed in a `static` now fails to compile with `*mut () cannot be shared between threads safely`.**
The change is intentional upstream: `NoopRawMutex` represents "single executor, no contention", which is fundamentally incompatible with global static storage that the language treats as `Sync` by default.
Fix: switch the static to `CriticalSectionRawMutex`. `ThreadModeRawMutex` would also satisfy `Sync` but is gated to `cfg(cortex_m)`, so it is unavailable on the RISC-V ESP32-C3/C6 targets.
For non-static, executor-local primitives (e.g. a signal owned by a task and passed by reference), `NoopRawMutex` continues to work and remains the zero-cost choice.

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

**Any edit to `Cargo.toml` (e.g. adding a new `[[example]]` entry) can cause Cargo to assign a new hash to the `esp-idf-sys` build directory, leaving two bootloader artifacts that the flash script refuses to resolve silently.**
`ensure-bootloader.sh` intentionally errors with `multiple IDF-built bootloaders found` rather than picking one arbitrarily, because choosing the wrong bootloader produces a silent boot-loop.
Fix: `cargo clean -p esp-idf-sys`, then re-run the flash command — the correct bootloader is rebuilt from scratch.

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

**`esp-idf-hal 0.46.2` `send_and_wait` panics in ISR context due to a bug in `EncoderWrapper`'s `From<rmt_encode_state_t>` conversion.**
The C RMT encoder returns bitwise-OR'd flag values (e.g. `RMT_ENCODING_COMPLETE | RMT_ENCODING_MEM_FULL = 0x03`).
The Rust `From<rmt_encode_state_t>` in `encoder.rs` only matches individual values (`0x00`, `0x01`, `0x02`) and panics on the `_` arm for any combined value.
The `send_and_wait` method wraps even C-native encoders like `BytesEncoder` in a Rust `EncoderWrapper` (via `into_raw`), inserting a Rust callback in the ISR path.
When the C encoder returns a combined state, the Rust callback panics in ISR context; the panic handler tries to print via `usb_serial_jtag_write`, which calls `_lock_acquire_recursive`, which aborts because recursive mutexes are illegal in ISR context.
Symptom: `abort() was called at PC 0x...` with stack trace showing `lock_acquire_generic` → `usb_serial_jtag_write` → `esp_vfs_write`.
Decoded via: `riscv32-esp-elf-addr2line -pfiaC -e target/.../examples/idf_c6_rainbow 0x40801e7d ...`
Fix: use `start_send` + `wait_all_done` directly with `BytesEncoder` (a `RawEncoder`), bypassing the `EncoderWrapper`.
This passes the C encoder handle directly to `rmt_transmit`, so the ISR calls the C encode function with no Rust wrapper.

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

**Passing `-s GITHUB_TOKEN=<any-value>` to `act` causes `act` to forward that value as HTTP Basic auth when cloning third-party actions from GitHub, breaking the download.**
`act` treats the `GITHUB_TOKEN` secret as a credential for all GitHub HTTP operations, not just the ones your workflow explicitly passes `${{ secrets.GITHUB_TOKEN }}` to.
If the token is invalid, GitHub rejects the clone with `authentication required: Invalid username or token. Password authentication is not supported for Git operations.`
The failure is counterintuitive: you'd expect a dummy token to be ignored rather than actively break unrelated operations.
Fix: omit `-s GITHUB_TOKEN=…` entirely when the workflow no longer references `${{ secrets.GITHUB_TOKEN }}`.
If a real token is needed for specific steps, scope it narrowly with a step-level `env:` block rather than passing it as a global `act` secret.

---

## Developer Tooling

**`.claude/hooks/just-enforcer.sh` blocks Bash commands whose first word matches a binary used by any `justfile` recipe — but does **not** block tools that no recipe wraps (e.g. `sed`, `perl`, `find`, `git`).**
This means batch text edits via `sed -i ''` or multi-line `perl -i -0pe` substitutions remain available even though direct `cargo` is funnelled through `just`.
Particularly useful during coordinated upstream API migrations that touch dozens of similar call sites at once (e.g. the 19-site `configure_tx` migration during the `esp-hal 1.1.0` upgrade).
A side effect worth knowing during settings cleanup: any `Bash(cargo *)` permission entries in `.claude/settings*.json` are functionally **dead** while the enforcer is active — they look granted but the hook intercepts before the permission resolver.

**Claude Code Stop hooks only support `type: "command"`; `type: "prompt"` is not a valid hook type and produces "Stop hook error: JSON validation failed".**
There is no built-in AI-evaluation hook type; the schema rejects unrecognised `type` values immediately.
Fix: use `type: "command"` pointing at a shell script, and call `claude -p` inside the script to preserve AI-based evaluation if required.

**The Stop hook `decision` field accepts `"approve" | "block"`, not `"allow"`.**
Outputting `{"decision": "allow"}` fails schema validation; the correct value to permit the session to end is `"approve"`.
Fix: use `{"decision": "approve"}` or simply exit 0 with no output to allow the stop.

---

## AVR WS2812 Driver: SPI Prerendered Encoding Limitation

**`rustyfarian-avr-ws2812` (SPI prerendered, `ws2812-pure::prerender_spi`) can produce stable white-ish output (no flicker, brightness scaling proportional, but every channel appears similarly lit) on the *same physical strip* that works correctly with `rustyfarian-esp-idf-ws2812` and `rustyfarian-esp-hal-ws2812`.**
Reproduced 2026-05-04 with both an Arduino Nano CH340 clone and a genuine Arduino Nano — ruling out clone-specific hardware quirks.
The 2 MHz SPI / 4-bits-per-WS2812-bit encoding emits `T0H = 0.5 µs` and `T1H = 1.5 µs` against a WS2812B nominal `T0H_max ≈ 0.55 µs` and `T1H_nom = 0.7 µs`; both the `T0H = 0.5 µs` value (right at the chip's "0/1" decision threshold) and the wildly out-of-spec `T1H` rely on chip tolerance, which varies between WS2812 / WS2812B / clone variants.
ESP32 RMT drivers don't hit this problem because they synthesize native WS2812 timing (`T0H ≈ 400 ns`, `T1H ≈ 700 ns`) — squarely inside spec.
Diagnostic dead-ends already ruled out: crystal frequency mismatch (`SerialClockRate::OscfOver4` vs `OscfOver8` produced no behavioural change), GRB color order, strip variant ("works on ESP, fails on AVR" rules out a wrong/dead strip), PulseEffect math, USB power supply (3V3 vs VIN/5V — no consistent improvement), cable length, and Arduino board (clone vs genuine).
Smoking-gun observation: with `NUM_LEDS = 1` (sending data for one LED only), LEDs 2 and 3 in the chain still flickered at minimum illumination — this is mechanical proof that LED 1 is not reliably consuming exactly 24 bits, so partial/extra bits leak into the next chip via the chain pass-through.
That rules in encoding-level bit-counting unreliability and rules out anything that would only affect color (color order, PulseEffect math, brightness scaling).
A working ESP setup is *not* a sufficient validation environment for the AVR driver: it must be validated on actual AVR hardware against the target strip type.

**Resolved (2026-05-04):** Adopted cycle-counted inline-`asm!` bit-bang as the recommended AVR backend, with the SPI prerendered backend retained as an opt-in alternative.
The asm pattern follows `Adafruit_NeoPixel`'s proven ATmega328P @ 16 MHz cycle counts (T0H = 4, T0L = 16, T1H = 13, T1L = 7, total 20 cy/bit).
Architectural decision recorded in [`docs/adr/007-avr-ws2812-driver-strategy.md`](adr/007-avr-ws2812-driver-strategy.md).

The production `Ws2812BitBang` driver landed in `rustyfarian-avr-ws2812` behind the `bitbang` cargo feature (const-generic over `PORT_ADDR` and `PIN_BIT`, runtime ports for any pin on PORTB / PORTC / PORTD on ATmega328P @ 16 MHz, internal `interrupt::free` wrapping).
Hardware-validated with `examples/avr-nano-rainbow/src/bin/bitbang_demo.rs` driving `ferriswheel::PulseEffect` — identical visible behaviour to the original spike (`bin/bitbang_spike.rs`, retained as a low-level reference).
`SmartLedsWrite` is implemented for both `Ws2812Spi` and `Ws2812BitBang` behind the `smart-leds-trait` feature, matching the sister ESP drivers.
