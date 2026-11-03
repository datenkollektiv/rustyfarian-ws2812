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

**A bare `embuild` build-dependency compiles only because `esp-idf-sys` turns on its `espidf` feature through feature unification.**
`embuild` 0.33 has no default features, and on the IDE target (`riscv32imac-unknown-none-elf`) `esp-idf-sys` is absent, so `build.rs` fails with `E0433: cannot find espidf in embuild`.
A runtime `CARGO_CFG_TARGET_OS` early return in `build.rs` cannot help, because the failure is at compile time.
Fix: declare `features = ["espidf"]` on the `embuild` build-dependency; `just check-ide` checks this path.
Verified 2026-10-13: `just check-ide` gets past `build.rs`, and `just check-idf` stays green.

**Deleting the cached `esp-idf-sys` build output is required when `sdkconfig.defaults` is first introduced.**
Cargo's `rerun-if-changed` does not fire for a file that didn't exist before, so the IDF build script is not re-run on the first commit of `sdkconfig.defaults`.
Fix: remove `target/<triple>/debug/build/esp-idf-sys-<hash>/` and rebuild. Subsequent edits to the file retrigger correctly.

**Building a workspace-excluded package out-of-tree needs the workspace root `Cargo.toml` symlinked too, not just its path dependencies.**
`examples/avr-nano-rainbow` depends on `../../crates/*`, and those crates use `version.workspace = true` inheritance, which requires a workspace root to resolve. Cargo searches upward from the **symlink** path rather than the resolved target, so symlinking only `crates/` into a temp dir never reaches the real repo root and fails with `failed to find a workspace root`.
Fix: symlink both `crates` and the root `Cargo.toml` into the temp directory at the same relative depth the package expects — see the `build-avr-example-upstream` recipe.
Verified 2026-08-13: the recipe builds clean from `mktemp -d` and leaves tracked files byte-identical.

**A command-line `-Z build-std=core` overrides the workspace `.cargo/config.toml`'s `[unstable] build-std`, but two `.cargo/config.toml` files merge their arrays instead of overriding.**
The workspace root sets `build-std = ["std", "panic_abort"]` for ESP-IDF; a child config that also set the key would merge to `["std", "panic_abort", "core"]`, which fails on AVR (no `std`).
Fix: keep `build-std` out of child configs and pass `-Z build-std=core` on the command line, as every `*-avr-*` recipe does — `examples/avr-nano-rainbow/.cargo/config.toml` carries a comment explaining exactly this.
Verified 2026-08-13: `just check-avr-target` runs clean from the workspace root with the root config's array in place.

**A freshly linked ESP-IDF binary can still report an old `Compile time:` in its boot log when the persistent `build.build-dir` cache is warm.**
The likely cause is the cached ESP-IDF CMake tree keeping its configure-time app-descriptor timestamp; the ELF itself is current.
Do not read that line as a stale flash — compare the boot log's `ELF file SHA256:` against `shasum -a 256` of the just-built binary.
Verified 2026-11-02: `idf_c3_rainbow` printed `Compile time: Sep 21 2026` while the Nov 2 ELF (`acbe0d4d8…`) matched the boot log; the timestamp's cause is inferred, not source-verified.

---

## Toolchain & Dependencies

**The `esp` toolchain (installed by `espup`) is required to compile `*-esp-espidf` targets; the stable toolchain does not know these targets.**
`cargo build --target riscv32imc-esp-espidf` with the stable toolchain produces `error[E0463]: can't find crate for core`.
Fix: prefix with `cargo +esp build …` or add `channel = "esp"` to a `rust-toolchain.toml`.

**`EffectError` from `ferriswheel` is `no_std` and does not implement `std::error::Error`, so the `?` operator cannot convert it to `anyhow::Error` in std contexts.**
Using `RainbowEffect::new(N)?` in an `anyhow::Result`-returning `fn main` fails at compile time with a trait bound error on `std::error::Error`.
Fix: use `.unwrap()` (or `.expect()`) for infallible construction validated at build time, matching the bare-metal pattern.

**A workspace `rust-version` above what the pinned AVR nightly reports makes Cargo refuse every AVR build, including path dependencies.**
`nightly-2025-04-27` identifies itself as `rustc 1.88.0-nightly`, and Cargo enforces `rust-version` against that number for every package in the graph, so a workspace-wide `rust-version = "1.95"` fails `just check-avr-target` and both AVR workflows with `rustc 1.88.0-nightly is not supported by the following package`.
The nightly cannot simply move: upstream `avr-hal` main and `avr-hal-template` pin the same `nightly-2025-04-27`, `avr-none` is Tier 3 and needs the still-unstable `-Z build-std`, and AVR `asm!` needs `#![feature(asm_experimental_arch)]`.
The bound covers the whole AVR build path, not just the AVR crate: overriding only `rustyfarian-avr-ws2812` to 1.88 still fails with `bunting@0.6.0 requires rustc 1.95`, because `bunting` is a path dependency that inherits the workspace value.
Fix: keep the workspace `rust-version` at the nightly's version and raise it per crate only where the ESP stack demands it (`rustyfarian-esp-hal-ws2812`, `rustyfarian-esp-idf-ws2812`); re-check upstream's `rust-toolchain.toml` before raising the floor.
Measured 2026-09-21 with a two-crate scratch workspace, then hit for real the same day when the first attempt at the `esp-hal 1.2` MSRV bump put 1.95 on the workspace.

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

**Async RMT frames longer than one channel's RAM (48 codes on C3/C6/S3, 64 on ESP32) garble or shift when the executor misses the refill deadline, because esp-hal 1.2.2 refills TX RAM in the transmit future's `poll`, not in the ISR.**
The async ISR only clears the interrupt and wakes the task, and the refill threshold is half the channel RAM, so the hardware wraps about 30 µs after the refill is due.
A late refill resends stale codes.
The blocking `wait()` refills in a busy loop and meets the deadline unless interrupts delay it.
Diagnostic signal: a frame that fits in channel RAM never garbles, and garbling scales with executor load (other tasks, Wi-Fi traffic) rather than with frame size alone.
Source-verified 2026-10-31 against esp-hal 1.2.2 (not hardware-verified).
Fix in principle: the critical-section path removes executor latency from the refill path by polling the future with a no-op waker, bounded by a timeout; it does not guarantee the waveform (flash-cache misses, cross-core flash stalls and high-priority Xtensa interrupt levels remain), so the workaround stays until esp-hal refills in the ISR.
Unmasked blocking polling (`LongFramePolicy::Poll`) has the same ~30 µs refill deadline, and radio or other interrupts can delay it.
Diagnostic signal: occasional `Error::Transmit` timeouts under Wi-Fi with `Ok` frames in between (downstream C3 bench, 12 LEDs, `c85a61a`, 2026-11-01: 23 `Transmit` in 8,419 frames, mostly while Wi-Fi associates, no `ChannelLost`).
Fix in principle: keep unmasked polling opt-in, and let the default blocking `CriticalSection` policy mask long frames instead, capped by the masked-time ceiling (1000 µs nominal, about 32 LEDs; the cap is an estimate, not a bound).
Verified against the code 2026-11-01; the earlier "not a fallback" wording predates that default.
Blocking inside `critical_section::with` (`set_pixels_slice_masked`) was reported clean with Wi-Fi active (downstream rgb-clock bench, ESP32-C3, 2026-11-01).
Mitigation to measure for flash-cache misses: `ESP_HAL_CONFIG_PLACE_RMT_DRIVER_IN_RAM=true`.

**With Wi-Fi active, critical-section long frames garbled on ESP32-C3 when the refill loop ran from flash; with the radio never started, the same frames were clean.**
Reported by the downstream rgb-clock bench (12 LEDs, ESP32-C3, 2026-11-01): modem sleep off still garbled, RAM placement clean, and the blocking driver inside `critical_section::with` clean even with its code in flash.
Fixed on the same bench at `3a1d3f8` (2026-11-01): clean with only the crate's loop in RAM and esp-hal's RMT code in flash, also clean with the esp-hal setting, and `set_pixels_slice_masked` clean; `3a1d3f8` also cut the clock reads from two per poll to one per 16 polls, so RAM placement and the cheaper loop are not separated.
Our no-radio bench (ESP32-C3, 12-LED ring, 2026-11-01): a cold cache alone (32 KB flash sweep before each frame, send time 440 → 865 µs) stayed visually clean.
Root cause is a hypothesis, not measured: the radio adds contention on shared resources (bus, SRAM, cache refills) that stretches the flash fetches of the refill loop.
Measured on the same bench at `735b506` (2026-11-01): with interrupts masked, the longest pass of the polling loop was 8–9 µs without the radio, 27–47 µs with the radio and esp-hal's RMT code in RAM, and 76–77 µs with it in flash; all frames were clean, so the slowdown is real but its position relative to the refill is not known.
Diagnostic signal: garbling appears only with the radio up and disappears with the RAM-placement setting.
Gaps between poll returns cannot bound refill latency, because the refill runs inside a poll; a 77 µs gap with clean frames (`735b506`) shows this.
Fix in principle: keep the refill loop small and in RAM; esp-hal's setting adds further margin and stays recommended for radio firmware, which the crate cannot detect. One clean bench is not a guarantee, and the hardware gate stays open.

**Dropping an unfinished async RMT TX future inside a critical section spins on the classic ESP32, because `TxGuard::drop` waits for the End event when the chip has no immediate TX stop.**
Source-verified 2026-10-31 against esp-hal 1.2.2 (`rmt.rs:1428-1440`, `3106-3113`; not hardware-verified): `rmt.has_tx_immediate_stop` is `false` on ESP32, so the drop spins until the hardware reaches the end marker, with interrupts masked if the drop happens there.
Diagnostic signal: a timeout on a long frame that never returns on ESP32 but returns on C3/C6/S3.

**A blocking single-shot RMT transmit in esp-hal 1.2.2 has no abort that returns the channel: `TxTransaction` offers only `poll()`, the unbounded `wait()`, or a drop that loses the channel.**
`stop()` exists only on `ContinuousTxTransaction`, while a failed `Channel::transmit` does hand the channel back as `Err((Error, Self))`.
Source-verified 2026-11-01 (`rmt.rs:1498-1545`, `1561-1600`, `1749-1766`); the driver's earlier "transmit failure consumes the channel" belief was wrong.
A late refill garbles the frame but still lets it end, because the writer refills on every Threshold event (`rmt/writer.rs`): downstream C3 bench, `c85a61a`, 2026-11-01, 0 `ChannelLost` in 8,419 `Poll` frames under Wi-Fi.
Fix in principle: after a timeout, keep polling unmasked for a second budget, and drop only if the frame still has not ended (`Error::ChannelLost`); see ADR 010.
Fix: keep the future in a slot declared outside the critical section, so it drops after the section ends; the spin then runs in the caller's interrupt state, which is masked only if the caller already masks it.

**`embassy-executor 0.10.0` reshaped the task-spawn API: `Spawner::spawn` now returns `()`, and `#[embassy_executor::task]` functions return `Result<SpawnToken, SpawnError>`.**
The old idiom `spawner.spawn(task()).unwrap()` no longer compiles; the unwrap moves *inside* the spawn argument: `spawner.spawn(task().unwrap())`.
`Spawner::must_spawn` does not exist on `embassy-executor 0.10.0` despite the compiler suggesting "method `spawn` with a similar name" — the explicit pattern is the supported one.
Prefer `spawner.spawn(task().expect("<task name> spawn token"))` over a bare `unwrap()` on embedded targets; named messages survive into release-mode panic prints and make field debugging tractable.

**`embassy-sync 0.8.0` made `NoopRawMutex` `!Sync`, so `Signal<NoopRawMutex, _>` / `Mutex<NoopRawMutex, _>` in a `static` now fail with `*mut () cannot be shared between threads safely`.**
Intentional upstream: `NoopRawMutex` ("single executor, no contention") is fundamentally incompatible with global statics, which the language treats as `Sync`. `ThreadModeRawMutex` would satisfy `Sync` but is `cfg(cortex_m)`-only, unavailable on RISC-V.
Fix: switch the static to `CriticalSectionRawMutex`. For non-static, executor-local primitives passed by reference, `NoopRawMutex` remains the zero-cost choice.

---

**Boot log visible but no `println!` output on an ESP32-C3 means esp-println writes to the native USB-Serial-JTAG port while the board's USB socket is a USB-UART bridge.**
The IDF bootloader logs to UART0, which a CH340/CP210x bridge carries, so the boot log appears and then the app is silent.
Diagnostic signal: the port is `/dev/cu.usbserial-*` (bridge) rather than `/dev/cu.usbmodem*` (native USB); `ioreg -p IOUSB` shows vendor 0x1A86 (WCH) or 0x10C4 (Silicon Labs).
Fix: build esp-println with the `auto` transport, which falls back to UART0 when no host is on the native port (verified on hardware 2026-10-31: a CH340-bridged C3 printed nothing with `jtag-serial` and logs normally with `auto`).

**An RMT end marker with a non-zero first half (`PulseCode::new(Low, 2800, Low, 0)`) holds the line low for that half before the transmission ends, so the WS2812 latch can ride inside the frame.**
`is_end_marker` only needs one zero length, and the End event fires after the first half has been sent, so an awaited short frame already includes its 280 µs latch.
Diagnostic signal: with the latch encoded and no software wait, back-to-back 1-LED frames still repeat at ≥ 308 µs; if the half were dropped they would repeat at ~30–100 µs.
Timing-verified on an ESP32-C3 on 2026-10-31 with `hal_c3_frame_sweep_async` (342–345 µs, 3 cycles); not yet confirmed with a logic analyser or on C6, S3 or the classic ESP32.

## espflash

**Connecting `espflash monitor` (including the one `just run` starts) resets the board, so a WS2812 animation freezes on its last frame for a moment and looks like a driver stall.**
The capture then starts with a fresh ROM boot (`rst:0x1 (POWERON)`), and holding RST freezes the ring the same way; BOOT (GPIO9) has no effect at runtime.
Diagnostic signal: a freeze that coincides with a monitor connect or a button press, while the app's own per-second log shows no gap.
Before reporting a visual stall, watch a power-cycled board with no monitor attached (confirmed on an ESP32-C3 on 2026-11-01).

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

**ESP-IDF rejects an RMT `mem_block_symbols` below the chip's block size; it rounds up only values above it.**
`rmt_tx.c` (v5.3.3) returns `ESP_ERR_INVALID_ARG` with `mem_block_symbols must be even and at least 64` on classic ESP32 and S2, whose blocks hold 64 words, while C3, C6, S3 and H2 hold 48.
A fixed 48 therefore could never create a channel on ESP32 or S2, and `idf_esp32_rainbow` had only ever been compile-verified; esp-idf-hal's own doc ("should be at least 64") is wrong in the other direction for the 48-word chips.
Fix: size the block from `esp_idf_hal::sys::SOC_RMT_MEM_WORDS_PER_CHANNEL` (a `u32` const in the bindgen output), as `Ws2812Rmt::new()` does since 2026-11-03.

**`esp_idf_hal::rmt::Pulse::new_with_duration` truncates to whole ticks, so a requested 350 ns at 10 MHz is emitted as 300 ns.**
`duration_to_ticks` is `nanos * hz / 1e9` in integer arithmetic with no rounding and no error (`src/rmt/pulse.rs`, 0.47.0, verified 2026-11-03).
Fix: request only widths that are whole multiples of the tick, which `bunting::ns_to_ticks` enforces at compile time, and never document the requested value as the emitted one.

**`esp-idf-hal 0.46.2` `send_and_wait` panics in ISR context: `EncoderWrapper::From<rmt_encode_state_t>` only matches single values (`0x00`, `0x01`, `0x02`) and hits the `_` arm on bitwise-OR'd flags like `COMPLETE | MEM_FULL = 0x03` that the C encoder legitimately returns.**
`send_and_wait` wraps even C-native encoders (`BytesEncoder`) in a Rust `EncoderWrapper`, inserting a Rust callback in the ISR path. The panic handler then calls `usb_serial_jtag_write` → `_lock_acquire_recursive`, which aborts (recursive mutexes are illegal in ISRs).
Symptom: `abort() was called at PC 0x...` with trace through `lock_acquire_generic` → `usb_serial_jtag_write`.
Fix: use `start_send` + `wait_all_done` directly with `BytesEncoder` (a `RawEncoder`), bypassing `EncoderWrapper` so the ISR calls the C encode function with no Rust wrapper.
Still present in `esp-idf-hal 0.47.0` (re-checked 2026-09-21 against `src/rmt/encoder.rs`); the workaround survives the 0.47 bump.

**`CONFIG_RMT_ISR_IRAM_SAFE=y` makes `Ws2812Rmt::new` fail with `ESP_ERR_INVALID_ARG` ("on_trans_done callback not in IRAM").**
`esp-idf-hal 0.47.0` `TxChannelDriver` always registers its flash-resident `handle_isr` as `on_trans_done`, and ESP-IDF v5.3.3 rejects non-IRAM callbacks when the ISR is IRAM-safe.
Source-verified 2026-11-01 (`esp-idf-hal` `src/rmt/tx_channel.rs:35-37`, `:216`; ESP-IDF `rmt_tx.c:506-508`); reproduced on the downstream rgb-clock C3 bench the same day, where the OTA image then rolled back.
With the default (`n`), a 12-LED ring stayed clean through a 1.6 MB OTA download on that bench (one run, no analyser).
Fix: leave the setting off until esp-idf-hal places its callback in IRAM; do not recommend it in the docs.

**The new RMT API (`TxChannelDriver`) registers ISR callbacks via `rmt_tx_register_event_callbacks`, requiring a larger FreeRTOS ISR stack than the legacy API.**
The default `CONFIG_FREERTOS_ISR_STACKSIZE=1536` overflows with the new API's callback overhead.
Symptom: `Guru Meditation Error: Stack protection fault` immediately after `WS2812RMT::new()` succeeds.
Fix: add `CONFIG_FREERTOS_ISR_STACKSIZE=4096` to `sdkconfig.defaults` and clean the IDF build cache.

**The default ESP-IDF main task stack (3584 bytes) overflows in debug builds when `set_pixels_slice` loops over 12+ WS2812 LEDs.**
`color_to_pulses` in `rustyfarian-esp-idf-ws2812` returns `[Pulse; 48]` (~192 bytes) per LED on the stack.
Debug builds retain larger temporaries and do not optimise stack usage, so 12 LEDs consumes ~2300+ bytes just for pulse arrays before accounting for other frames.
The symptom is a `Guru Meditation Error: Core 0 panic'ed (Stack protection fault)` immediately after `WS2812RMT init OK`.
Fix: add `sdkconfig.defaults` at the workspace root with `CONFIG_ESP_MAIN_TASK_STACK_SIZE=8192`.

**A 12-LED frame is 317–374 µs of WS2812 wire time, so neither driver's logged send time measures the wire.**
`BytesEncoderConfig` uses 1.1 µs per zero bit (300 + 800 ns; esp-idf-hal 0.47 truncates the requested 350 ns to 3 ticks at 10 MHz, verified 2026-11-03) and 1.30 µs per one bit (700 + 600 ns), so 288 pulse codes span 317–374 µs depending on the data.
The esp-hal driver logs ~392 µs for that frame, which is the wire time plus encoding overhead.
The ESP-IDF driver logs ~4.3 ms, which is the fixed overhead in the third entry below, not a longer frame.
Arithmetic verified against the code 2026-11-03.

**ESP-IDF RMT refill stalls never reach the error count: `wait_all_done(None)` returns `Ok` for a garbled frame and blocks indefinitely when the refill never recovers.**
The refill runs from ESP-IDF's RMT ISR, which is not IRAM-safe and runs at low or medium priority, so flash-cache or task activity can delay it.
Diagnostic signal: log the per-frame send time and compare `max` with `mean`; a statistics line that stops appearing is a total stall, not a quiet ring.
Observed 2026-11-02/03 on the C3 bench: `idf_c3_rainbow` flickered on one LED of twelve, while the same ring stayed clean under `hal_c3_pulse` and `hal_c3_rainbow_comet_async`; the cause is not confirmed.

**`Ws2812Rmt::set_pixels_slice` costs a fixed ~3.6 ms on ESP-IDF regardless of frame length, so its send time cannot be read as wire time.**
Measured on the C3 bench 2026-11-03: 2 LEDs 3.92 ms and 12 LEDs 4.33 ms, against ~0.39 ms for the same 12-LED frame on the esp-hal driver; only the ~0.4 ms difference tracks the wire time.
A 2-LED frame is 48 symbols and needs no refill, yet still pays the overhead, so the overhead itself is not per-refill work.
Hypothesis, not measured: the fixed part is task wake-up latency in `wait_all_done`; refill starvation is neither observed nor ruled out, because a brief underrun can corrupt symbols without changing how long the frame takes.
`std::thread::sleep(Duration::from_millis(50))` also returns after ~46.5 ms on the same build, which is why the loop period still comes out at 50 ms.

---

## Scripts & Shell

**`set -eo pipefail` causes silent script exit when `ls` targets an unmatched glob inside a command substitution.**
`ls glob-pattern 2>/dev/null | head -1` exits non-zero when the glob matches nothing; with `pipefail`, the pipeline's non-zero exit propagates through `var=$(...)`, and bash exits the script before any `if [ -z "$var" ]` guard runs.
The failure is invisible: the script exits with code 1 and no output, so the parent script and `just` recipe fail with only a line number.
Fix: append `|| true` to the pipeline: `var=$(ls glob 2>/dev/null | head -1 || true)`.

---

## Rust Syntax Gotchas

**Widening a parameter from `(u8, u8, u8)` to `impl Into<RGB8>` keeps literal tuples such as `(0, 0, 255)` compiling.**
`rgb 0.8` implements `From<(R, R, R)> for RGB<S> where R: Into<S>`, and `u8` is the only integer type with `Into<u8>`, so the literals infer to `u8` instead of falling back to `i32`.
Verified in a scratch crate on 2026-11-03 for literals, a `u8` variable and a `const (u8, u8, u8)`; this is why `pennant::PulseEffect::update` could take `RGB8` without breaking the six downstream tuple call sites.

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

**`act` never fetches `actions/checkout` — it substitutes a `docker cp` of the working tree (verified 2026-09-24).**
The step reports `Success - Main Checkout` while the action's own code is never downloaded or executed, so a `checkout` version bump cannot be validated locally no matter which recipe runs.
Consequence: `just act-*` proves nothing about a checkout upgrade; only a real push does.

---

## Test Infrastructure

**The `rustyfarian-esp-hal-ws2812` `#[cfg(test)]` module cannot build on a macOS host, so its unit tests never run.**
`esp-hal`'s `#[link_section = ".rwtext.interrupt"]` is not a valid Mach-O section specifier, and `esp-sync` cannot find the `riscv` crate (reproduced with `just test-crate rustyfarian-esp-hal-ws2812`, 2026-11-01).
`just test` and `just verify` skip this crate, so a green run says nothing about its tests.
Fix: put routing and timing logic in `const fn`s guarded by `const _: () = assert!(..)`, which every target build checks.

---

## CI Coverage

**A feature that only a host recipe enables is never compiled for the real target, however often CI is green.**
The AVR crate's `smart-leds-trait` feature (and its 768-byte stack adapter) was checked with `--features bitbang,smart-leds-trait` on the host only; the two `avr-none` recipes built `bitbang` alone, so no run ever compiled the trait impls for AVR until 2026-11-03.
Fix: when adding a feature to a target-specific crate, add it to the recipe that builds for that target, not only to the host `*-all-features` one.

**A `cargo check` or `cargo build` of a library-only crate never links, so neither can catch linker, ABI, or `-C target-cpu` breakage.**
`rustyfarian-avr-ws2812` declares no `[[bin]]` and emits an rlib, so `cargo build --target avr-none` passes while the AVR linker goes entirely unexercised — `build` buys nothing over `check` here, despite the usual advice that it does.
The only real link coverage is `just build-avr-example-all-bins`, which builds the workspace-excluded `examples/avr-nano-rainbow` package into four ELF executables (~15s).
Fix: when a CI job's purpose is catching toolchain rot, gate the package that produces binaries, not the library crate — gating the driver alone looks like AVR coverage and is not.

**Dev-dependencies are invisible when deriving CI path filters from `[dependencies]`, yet any `--examples` pass pulls them in.**
`ferriswheel` appears only under `[dev-dependencies]` of `rustyfarian-esp-hal-ws2812`, but 14 of that crate's 17 examples use it, and both `check-hal-c3` and `check-hal-xtensa` run a `--examples` invocation.
A path filter built from the runtime dependency list alone therefore lets a `ferriswheel` change break the ESP jobs without ever triggering them.
Fix: derive workflow `paths:` from dependencies *and* dev-dependencies whenever the job builds examples, tests, or benches.

**An action's Node runtime is only knowable from `runs.using` in its `action.yml` at the tag in use — release notes and summaries get it wrong (verified 2026-09-24).**
Floating major tags move underneath you: during the Node 20 deprecation, `Swatinem/rust-cache@v2` already declared `node24` and needed no bump, while pinned `actions/checkout@v4` did — and a research summary dated that rust-cache migration to March 2024, before Node 24 existed.
The runner's deprecation warning lists only the actions it flags, so it under-reports: composite actions (`dtolnay/rust-toolchain`, `taiki-e/install-action`, `esp-rs/xtensa-toolchain`) are exempt, and a node20 action in a path-filtered job that did not run is simply absent.
Fix: read `https://raw.githubusercontent.com/<owner>/<repo>/<tag>/action.yml` for every action in `.github/workflows/`, not just the ones named in the warning.

**A `rust-version` floor does not protect a pinned nightly, because `1.88.0-nightly` satisfies `rust-version = "1.88"` while lacking features that 1.88.0 stabilised.**
`cross-target-avr-upstream` went red 2026-09-07 through 2026-09-21 on `encoding_rs 0.8.40` (published on exactly that first date, reached via `avr-hal → yaml-rust2`), which declares `rust-version = "1.88"` and calls `slice::as_chunks` — stabilised in Rust 1.88.0 on 2025-06-26, two months after the pinned `nightly-2025-04-27`.
Cargo therefore selects the crate and the compiler then rejects it, so the MSRV gate gives no warning; the same quirk worked in our favour during the `esp-hal 1.2` bump, where `1.95.0-nightly` satisfied `rust-version = "1.95"`.
Fix: keep a lockfile for anything built on a pinned nightly. A nightly older than a stabilisation date is a floor that `rust-version` cannot express.

**An "upstream early-warning" job that deletes its lockfile tests every crate in the graph, not the upstream it names.**
`build-avr-example-upstream` used to `rm -f Cargo.lock`, so three weeks of red actually reported unrelated registry churn while `avr-hal` itself was fine — and the noise meant nobody read the signal.
Removing the `rev = ` line is sufficient on its own: it changes the git source spec, so the locked `arduino-hal` / `atmega-hal` / `avr-hal-generic` entries no longer match and Cargo refetches just those three, leaving registry deps at their locked versions.
Fix (2026-09-25): drop the `rm -f Cargo.lock`. Verified by a run that compiled `avr-hal` at upstream `e0b0105b` while holding `encoding_rs` at the locked 0.8.35.

---

## Release & crates.io

**There is no known read-only way to prove a crates.io publish token is valid; every candidate test gives a wrong answer (all verified 2026-09-25).**
`GET /api/v1/me` returns 403 for scoped tokens that publish fine, so a failure there is not evidence of a dead credential — it produced a false blocker and an unnecessary token re-mint during the v0.7.0 pre-flight.
`cargo owner --list <crate>` is the opposite trap: it succeeds with a deliberately garbage token because `/crates/:name/owners` is a public endpoint, and refuses only when *no* token is present — so it tests presence and reports a false green for validity.
`cargo publish --dry-run` authenticates not at all; it stops before upload and passes with no credentials.
Fix: check presence only (`just doctor` prints `crates.io present`) and treat the first `cargo publish` of a staged release as the auth test — stage the pure crates first so a bad token fails before anything is uploaded.

**`CARGO_REGISTRY_TOKEN` in the environment silently overrides `~/.cargo/credentials.toml`.**
`cargo login` appears to succeed and changes nothing, so a stale exported token keeps winning.
Fix: unset the variable, or export the new value rather than running `cargo login`.

**Pinning a hardware sign-off to a commit SHA does not survive the GitHub flow (changed 2026-09-25).**
Rebase-and-squash on merge rewrites the SHA, leaving the record pointing at a commit on no branch that will be garbage-collected — it reads as an invalidated validation even when no code changed.
Fix: anchor validation records to the released version and the pinned dependency versions, and state invalidation in content terms (a change to the driver crate, the named example, or a pinned `esp-*` version).

**A comment-only edit to an example changes its firmware binary when it adds or removes lines.**
Every `.unwrap()` panic embeds a `core::panic::Location` with its source line number, so shifting the code below a doc comment changes the image even in release builds.
A "docs only" polish after a hardware run therefore silently invalidates the sign-off for that binary.
Fix: record the binary's `sha256` with the sign-off, then rebuild after any polish and compare; keep a comment rewrite within the same line count if the binary must stay identical.
Verified 2026-10-14: adding two lines to `hal_s3_pulse_async.rs`'s header changed its sha256, and a same-line-count rewrite restored the flashed hash byte for byte.

---

## Developer Tooling

**`.claude/hooks/just-enforcer.sh` blocks Bash commands whose first word matches a binary wrapped by any `justfile` recipe but allows tools no recipe wraps (e.g. `sed`, `perl`, `find`, `git`).**
Batch text edits via `sed -i ''` and `perl -i -0pe` therefore remain available even though direct `cargo` is funnelled through `just`.
Side effect during settings cleanup: any `Bash(cargo *)` permission entries are functionally **dead** while the enforcer is active — they look granted but the hook intercepts before the permission resolver runs.

**The `block-git-mutations` hook rejects `git worktree add`, so a clean-HEAD reproduction needs `git archive` instead.**
Checking whether a failure predates the working-tree changes is otherwise a stash or worktree, both blocked (verified 2026-11-01).
Fix: export HEAD to a scratch directory and run the recipe there.

```bash
rm -rf /tmp/head && mkdir /tmp/head && git archive HEAD | tar -x -C /tmp/head
```

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
The 2 MHz SPI / 4-bits-per-WS2812-bit encoding emits `T0H = 0.5 µs` (right at the chip's 0/1 decision threshold) and `T1H = 1.5 µs` (well outside the WS2812B nominal `T1H ≈ 0.7 µs`); correctness then depends on per-chip tolerance which varies between WS2812 / WS2812B / clone variants. ESP RMT synthesises native timing (`T0H ≈ 300 ns` (300 ns since 2026-11-03; 400 ns before), `T1H ≈ 700 ns`) and does not hit this.
Smoking-gun diagnostic: with `NUM_LEDS = 1`, downstream LEDs in the chain still flicker — proof that LED 1 is not consuming exactly 24 bits, so partial/extra bits leak via the pass-through. This confirms encoding-level bit-counting unreliability as the cause and rules out colour-only theories (GRB order, PulseEffect math, brightness scaling). Crystal mismatch, power supply, cable length, and clone-vs-genuine board were also ruled out (2026-05-04).
**Lesson: a working ESP setup is *not* sufficient validation for the AVR driver — AVR hardware against the target strip type is required.** Resolution (cycle-counted inline-`asm!` bit-bang as the recommended backend) is documented in [`docs/adr/007-avr-ws2812-driver-strategy.md`](adr/007-avr-ws2812-driver-strategy.md).
