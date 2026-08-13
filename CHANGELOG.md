# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- `pennant`: `RgbGpioLed` adapter (behind the `hal` feature) that drives a discrete, non-WS2812 RGB LED over three separate `embedded-hal` 1.0 `OutputPin`s, switching each channel on/off from an `RGB8` colour via a per-channel brightness threshold (eight on/off colours, not analog colour mixing); a new `Polarity` enum selects common-anode (`ActiveLow`) or common-cathode (`ActiveHigh`, default) wiring, covering boards like the Cheap Yellow Display (ESP32-2432S028R) whose onboard RGB LED is active-low on GPIO 4/16/17
- `pennant`: `channel_on(value, threshold)` — pure per-channel threshold helper (strict greater-than) alongside the existing `exceeds_threshold`
- `pennant`: `RgbPwmLed` adapter (behind the `hal` feature) — drives a discrete, non-WS2812 RGB LED over three `embedded-hal` 1.0 `SetDutyCycle` PWM channels, mapping each `RGB8` component onto a duty-cycle fraction for **smooth analog colour mixing** (true brightness control, unlike `RgbGpioLed`'s eight on/off colours); shares the `Polarity` flag with `RgbGpioLed` for common-anode (`ActiveLow`) / common-cathode (`ActiveHigh`, default) wiring
- `rustyfarian-esp-idf-ws2812`: two discrete-RGB-LED examples, both using the Cheap Yellow Display's RGB GPIO numbers 4/16/17 and defaulting to an external common-cathode LED (`ActiveHigh` — set `ActiveLow` for the CYD's common-anode onboard LED). `idf_esp32_rgb_cycle` drives `pennant`'s `RgbGpioLed`, cycling the eight on/off colours for on-hardware verification (`just build-example-esp32-rgb`, `just run idf_esp32_rgb_cycle`, example-only `rgb-gpio` feature); `idf_esp32_rgb_pulse` fades the LED in and out with `PulseEffect` via `RgbPwmLed` over three ESP-IDF LEDC PWM channels, so the effect renders as a smooth brightness fade rather than a blink (`just build-example-esp32-rgb-pulse`, `just run idf_esp32_rgb_pulse`, example-only `rgb-pwm` feature). Both features pull in `pennant/hal`
- Build tooling: persistent Cargo `build.build-dir` split for IDF builds (adopted; see [`docs/features/separate-build-environments-v1.1.md`](docs/features/separate-build-environments-v1.1.md)) — the bulky `esp-idf-sys` CMake tree relocates to `~/Library/Caches/rustyfarian-cargo-build/<workspace-hash>` while IDF final artifacts stay on the RAM disk. Resolved by `scripts/idf-build-dir.sh` and threaded through every IDF recipe. Override the cache location with `RUSTYFARIAN_IDF_BUILD_DIR`. Hardware-flash confirmation is still outstanding
- Build tooling: `just clean-idf-cache` removes the relocated persistent IDF build cache, and `just idf-build-dir-info` prints the resolved build-dir / `--config` flag / glob and whether the cache is materialised
- Build tooling: `just clean-idf-stale` — removes superseded `esp-idf-sys-*` build directories while keeping the newest per target, fixing the `multiple IDF-built bootloaders found` flash error without the full rebuild that `clean-idf-cache` forces. A dependency bump rehashes `esp-idf-sys` for every IDF target at once but each target only grows a second directory when next built, so the error recurs per architecture; this recipe sweeps them all
- CI: cross-target workflows gating all three embedded targets, which previously had **no** CI coverage at all — `cross-target-riscv.yml` (ESP32-C6 `riscv32imac` via `just check-hal`, plus ESP32-C3 `riscv32imc` via the new `just check-hal-c3`), `cross-target-avr.yml` (`just check-avr-target` / `check-avr-target-bitbang`, plus `just build-avr-example-all-bins` — the driver crate is lib-only, so only the example binaries exercise the AVR linker and `-C target-cpu=atmega328p`), and `cross-target-xtensa.yml` (`just check-hal-xtensa` on the Xtensa Rust toolchain, pinned to 1.97.0.0 and cached). Each is path-filtered to its own crates so the expensive Xtensa install stays off unrelated PRs; all three call existing `just` recipes so CI and local runs stay identical. See [`docs/features/cross-target-ci-v1.md`](docs/features/cross-target-ci-v1.md)
- CI: `cross-target-avr-upstream.yml` — a weekly (and manually dispatchable) job that drops the `arduino-hal` pin and builds the AVR example against upstream `avr-hal` main, via the new `just build-avr-example-upstream`. Pinning made PR builds reproducible but would have hidden upstream breakage, which the AVR gate exists partly to catch; this restores that signal without sacrificing reproducibility. A failure here means upstream moved, not that this repo is broken
- `examples/avr-nano-rainbow`: `arduino-hal` is now pinned to `rev = "e5c8f37fe484…"` and the package's `Cargo.lock` is committed (via a `.gitignore` negation), making the AVR CI job reproducible. `avr-hal` publishes nothing to crates.io and ships no tags or releases, so a commit SHA is the only available ref — and it is the same SHA the official `avr-hal-template` pins, so the previous bare `git = …` dependency was a deviation from upstream's own documented practice. `just build-avr-example-all-bins` now passes `--locked` so CI fails loudly if the manifest and lockfile drift apart
- `justfile`: `check-hal-c3` — checks the esp-hal driver on the ESP32-C3 target (`riscv32imc`, no atomics), including its examples. `check-hal` covers the C6/`riscv32imac` default-feature configuration, so the C3 build had no equivalent check
- `rustyfarian-esp-hal-ws2812`: `hal_c6_onboard_pulse` example — pulses the onboard SK68XXMINI LED on **GPIO8** of an ESP32-C6-DevKitC-1, requiring no external wiring. Exists as a standing regression guard: GPIO8 has a documented history of hanging in `txn.wait()` on the first transmit (fixed upstream in `esp-hal 1.1.0` with no workaround on our side), and every other `hal_c6_*` example targets GPIO18, so the pin had no coverage. Build with `just build-example hal-ws2812 hal_c6_onboard_pulse`, flash with `just flash hal_c6_onboard_pulse`

### Changed

- `rustyfarian-esp-hal-ws2812`: bumped the pinned `esp-hal` from `=1.1.0` to `=1.1.2` — patch-level only. Upstream guarantees no breaking changes within a patch series, and the companion crates (`esp-rtos`, `esp-println`, `esp-bootloader-esp-idf`) and Embassy pins are unchanged, so this is an isolated bump rather than a coordinated release wave
- `rustyfarian-esp-hal-ws2812`: `esp-hal 1.1.1` includes an RMT fix ("pulse length now supports maximum values on both phases") in the peripheral this driver uses. Our timing constants (`T0H=4, T0L=8, T1H=7, T1L=6` ticks) sit far below the affected range, and no behaviour change was observed
- `rustyfarian-esp-hal-ws2812`: the bump is hardware-validated on an ESP32-C6 across the blocking, async, Embassy multi-task, `smart-leds` and GPIO8 onboard paths, and on an ESP32-C3 via `hal_c3_pulse` — the latter closing a hardware-validation gap open since 2026-04-29. ESP32-WROOM-32 (Xtensa) remains compile-verified only. See [`docs/features/archive/esp-hal-stack-upgrade-august-2026-v1.md`](docs/features/archive/esp-hal-stack-upgrade-august-2026-v1.md)
- `rustyfarian-esp-idf-ws2812`: bumped the pinned `embuild` build-dependency from `=0.33.1` to `=0.33.3` (patch-level; build-time only, no runtime footprint)
- `justfile`: `check-hal-xtensa` now enables `pennant` in both invocations, mirroring the crate's `default = ["esp32c6", "unstable", "pennant"]` feature set. It previously checked `esp32,unstable` only, so the `pennant` `StatusLed` / `AsyncStatusLed` impls were never compiled for the Xtensa target — a gap that started to matter once the recipe became a CI gate. Verified clean on `xtensa-esp32-none-elf`
- `just clippy-all` now excludes `rustyfarian-avr-ws2812` from the ESP-IDF-target lint run, consistent with `build-all` and `check-all`; the AVR crate is still linted on the host target via `just clippy`
- Build tooling: `just clean-idf` now also sweeps the relocated `esp-idf-sys` tree in the persistent build cache (keeping the legacy in-target-dir sweep for pre-split builds); `just doctor` reports the IDF build-dir and whether this workspace's sharded cache has been materialised
- Build tooling: `justfile` recipe descriptions trimmed so `just --list` renders one scannable line per recipe — widest row 286 → 99 columns, and no row now exceeds 100 (63% did before). All 72 recipes remain documented; none were renamed or removed. `just` renders only the *last* comment line above a recipe, so the full detail stays in the file for whoever edits it

## [0.6.0] - 2026-05-20

### Added

- **First crates.io publication** of `rustyfarian-avr-ws2812`, `rustyfarian-esp-idf-ws2812`, and `rustyfarian-esp-hal-ws2812` — all three driver crates are now available on crates.io alongside the pure-logic trio (`bunting`, `pennant`, `ferriswheel`)
- `rustyfarian-esp-idf-ws2812`: `Ws2812Rmt::new_with_channel_config(led, TxChannelConfig)` — new constructor for callers who need to override RMT channel parameters (memory block size, DMA mode, etc.) for ESP32 variants beyond C3/C6
- `rustyfarian-esp-hal-ws2812`: confirmed Xtensa ESP32 / WROOM-32 target (`xtensa-esp32-none-elf`) compiles clean under `esp-hal 1.1.0` with the `esp` toolchain (Xtensa Rust 1.95.0.0); `just check-hal-xtensa` added to the justfile for ongoing CI coverage

### Changed

- `rustyfarian-esp-idf-ws2812`: renamed `WS2812RMT` to `Ws2812Rmt` (RFC 0430 UpperCamelCase, consistent with `rustyfarian-esp-hal-ws2812`); `WS2812RMT` remains as a `#[deprecated(since = "0.6.0")]` type alias and will be removed in 0.7.0
- `rustyfarian-esp-idf-ws2812`: `Ws2812Rmt::new()` now documents that its default `memory_block_symbols: 48` is specific to ESP32-C3/C6 (48-symbol blocks, only 2 TX channels); delegates to `new_with_channel_config` internally

## [0.5.0] - 2026-05-06

### Added

- `ws2812-pure`: `grid` module with `GridBuffer` (`fill`, `set_pixel`, `set_brightness`, `as_slice`), `GridLayout` (`RowMajor`, `ColumnMajorBottomUp`), `GAMMA_2_0` LUT, and `apply_brightness_gamma` helper for rectangular LED matrices, extracted from rustbox-rgb-puzzle matrix firmware
- `rustyfarian-avr-ws2812` — WS2812 LED driver for AVR/ATmega328P using SPI prerendered encoding via `embedded-hal 1.0` `SpiBus`
- `rustyfarian-avr-ws2812`: `Ws2812BitBang<P, PORT_ADDR, PIN_BIT>` — cycle-counted inline-`asm!` bit-bang backend (feature `bitbang`), the recommended default per [ADR 007](docs/adr/007-avr-ws2812-driver-strategy.md). Const-generic over port-register address and pin bit; supports any pin on PORTB / PORTC / PORTD on ATmega328P at 16 MHz; wraps the asm loop in `avr_device::interrupt::free` internally
- `rustyfarian-avr-ws2812`: `smart_leds_trait::SmartLedsWrite` impl for both `Ws2812Spi` and `Ws2812BitBang` (feature `smart-leds-trait`) — matches the sister ESP drivers for ecosystem parity
- `examples/avr-nano-rainbow/src/bin/bitbang_demo.rs` — production bit-bang demo using `Ws2812BitBang` with `ferriswheel::PulseEffect`. Companion `just flash-avr-bitbang-demo` recipe
- `examples/avr-nano-rainbow/src/bin/spi_rainbow.rs` — SPI-prerendered rainbow as a diagnostic / comparison binary, with NUM_LEDS=12 and a prominent header documenting the known white-ish failure mode (per ADR 007). Companion `just flash-avr-spi-rainbow` recipe
- `ferriswheel`: oversized-buffer acceptance tests for all 14 effects — confirms buffers larger than `num_leds` are accepted and excess entries are not modified
- `led-effects`: `AsyncStatusLed` trait — async counterpart of `StatusLed` for drivers with async `set_color`; `NoLed` implements it
- `rustyfarian-esp-hal-ws2812`: implements `AsyncStatusLed` for `Ws2812Rmt<'d, Async, N>` behind `async` + `led-effects` features
- `hal_c3_pulse_async`, `hal_c6_pulse_async`, `hal_esp32_pulse_async` — async blue pulse examples using `AsyncStatusLed` for ESP32-C3, C6, and WROOM-32
- `ferriswheel`: `smart-leds-compat` feature — optional `smart-leds-trait` dependency with a compile-time type-identity assertion that fails the build if the two crates resolve to incompatible `rgb` versions (no runtime impact)
- `ferriswheel`: `FireEffect::with_wrap(bool)` — circular heat diffusion for ring displays; heat wraps from tip back to base, eliminating the cold seam
- `ferriswheel`: `FireEffect::with_base_range(usize)` — configurable ignition zone width; default `num_leds.min(3)`, clamped to `1..=num_leds`
- `rustyfarian-esp-hal-ws2812`: `hal_c6_multitask_async` example — multi-task Embassy demo with cooperating render and button tasks via `embassy-sync` primitives (Chromatic Clash M1)

### Changed

- **Renamed `ws2812-pure` → `bunting`** to lift the pure-logic crate out of the crowded `ws2812-*` crates.io namespace and align it with the fairground naming family (`ferriswheel`, `pennant`). No behaviour or API changes. Migration: replace `ws2812-pure = ...` with `bunting = ...` in your `Cargo.toml`, and `use ws2812_pure::...` with `use bunting::...` in your source. Decision recorded in [`docs/features/archive/crates-io-publication-v1.md`](docs/features/archive/crates-io-publication-v1.md).
- **Renamed `led-effects` → `pennant`** to align with the fairground naming family (`ferriswheel`, `bunting`) and to claim a name that is free on crates.io — the originally chosen `lantern` was found taken on re-verification. `pennant` (a single triangular flag) pairs naturally with `bunting`, which is literally a string of pennants. No behaviour or API changes. Migration: replace `led-effects = ...` with `pennant = ...` in your `Cargo.toml`, and `use led_effects::...` with `use pennant::...` in your source. The Cargo feature flag on `rustyfarian-esp-hal-ws2812` and `rustyfarian-esp-idf-ws2812` was renamed in lockstep — `--features led-effects` callers must switch to `--features pennant`. Decision recorded in [`docs/features/archive/crates-io-publication-v1.md`](docs/features/archive/crates-io-publication-v1.md).
- `ferriswheel`: all 14 effect structs now derive `PartialEq`, enabling direct `assert_eq!` comparisons in tests
- `examples/avr-nano-rainbow/src/main.rs` — default example now drives the bit-bang backend (the recommended path per ADR 007). The previous SPI rainbow content moved to `bin/spi_rainbow.rs` as a diagnostic comparison. `just flash-avr-example` now runs the bit-bang rainbow
- `rustyfarian-avr-ws2812`: dropped the `avr-device` dependency. The bit-bang backend now uses raw inline `cli` + `SREG` save/restore asm for its critical section instead of `avr_device::interrupt::free`. This removes the deprecated `bare-metal` crate (RUSTSEC-2026-0110) and the GPL-3.0 `atdf2svd` build-tool from the workspace dep graph entirely. No behaviour change for users.
- `rustyfarian-esp-hal-ws2812`: `esp-println` moved from dev-dependency to optional dependency with per-chip feature forwarding — fixes cross-chip example builds (C3/ESP32 examples no longer conflict with C6's `esp-println` features)
- `rustyfarian-esp-hal-ws2812`: all blocking examples updated from `Ws2812Rmt::<N>` to `Ws2812Rmt::<_, N>` to match the v0.4.0 `Dm: DriverMode` type parameter
- `esp-hal` upgraded `1.0.0` → `1.1.0`; coordinated bump of the April 2026 monorepo wave: `esp-rtos 0.2 → 0.3`, `esp-bootloader-esp-idf 0.4 → 0.5`, `esp-println 0.16 → 0.17`
- `embassy-executor 0.9 → 0.10`, `embassy-sync 0.7 → 0.8`, `embassy-time 0.5.0 → 0.5.1` — aligned with `esp-rtos 0.3` transitive resolution
- All `hal_*` examples migrated to the new `esp-hal 1.1.0` RMT API: `configure_tx(&config).unwrap().with_pin(pin)` chained pattern (was `configure_tx(pin, config).unwrap()`)
- `hal_c6_multitask_async`: migrated to `embassy-executor 0.10` task-spawn pattern (`spawner.spawn(task().expect("…"))` — task functions now return `Result<SpawnToken, SpawnError>`); `EFFECT_SIGNAL` switched from `NoopRawMutex` to `CriticalSectionRawMutex` because `embassy-sync 0.8` made `NoopRawMutex` `!Sync` and it can no longer appear in a `static`
- `docs/esp-hal-1.0.0-version-matrix.md` renamed to `docs/esp-hal-version-matrix.md` and gained a current-state header for the 1.1.0 wave

### Fixed

- `ferriswheel`: `FireEffect` now picks the base-spark index with rejection sampling instead of `rng_byte() % base_range`, removing modulo bias for non-power-of-2 `with_base_range` values (negligible at the default 1–3, noticeable for wider bases)
- `ferriswheel`: corrected the `FireEffect.heat` doc comment — peak heat (255) maps to bright yellow via `fire_color`, not white
- `ferriswheel`: `MeteorEffect::new()` now clamps the default `tail_length` to `num_leds - 1`, preventing a subtract overflow when `num_leds < 7`
- Workspace `Cargo.toml`: `esp-hal` constraint comment claimed "exact 1.0.0 pinning" but the bare `"1.0.0"` constraint is `^1.0.0` to Cargo. Switched to true exact pinning with `"=1.1.0"` (and `"=…"` for every coordinated companion crate) and updated the comment to match — the workspace ignores `Cargo.lock`, so caret drift here was the actual risk
- `rustyfarian-esp-hal-ws2812`: GPIO8 RMT blocking transmit hang on ESP32-C6-DevKitC-1 (`Channel<Blocking, Tx>::transmit(&buffer).wait()` would lock up indefinitely on the onboard SK68XXMINI LED) is resolved by the `esp-hal 1.1.0` upgrade — confirmed by hardware retest (2026-04-29). The bare-metal driver can now drive the onboard LED on GPIO8 directly

## [0.4.0] - 2026-03-13

### Added

- `ferriswheel`: `RainbowCometEffect` — orbiting comet with a hue-cycling tail; each tail LED steps further along the color wheel with decreasing brightness; configurable hue, saturation, brightness, hue step, tail length, speed, direction, and decay
- `ferriswheel`: `KnightRiderEffect` — dual-headed scanner where two heads start at opposite ends, sweep toward each other, cross in the middle, and reverse independently at each end; configurable color, speed, tail length, and decay
- `hal_c6_knight_rider` and `idf_c6_knight_rider` examples for ESP32-C6
- `rustyfarian-esp-hal-ws2812`: `async` feature flag — enables async `set_pixel` and `set_pixels_slice` on `Ws2812Rmt<'d, Async, N>` using `esp-hal`'s native async RMT channel and `esp-rtos` as the Embassy executor; `Ws2812RmtBlocking` type alias provided for code that wants to avoid writing the `Blocking` type parameter explicitly
- `hal_c6_rainbow_comet_async` example — async animation loop using Embassy `Timer` to yield between frames

### Changed

- `rustyfarian-esp-idf-ws2812`: migrated from legacy RMT API (`rmt-legacy` feature) to new `esp-idf-hal 0.46` RMT API using `BytesEncoder`; `WS2812RMT::new()` no longer requires an `RmtChannel` parameter (**breaking**)
- `rustyfarian-esp-hal-ws2812`: `Ws2812Rmt` gains a `Dm: DriverMode` type parameter (`Ws2812Rmt<'d, Dm, N>`); existing blocking code can use the `Ws2812RmtBlocking<'d, N>` type alias or write `Ws2812Rmt<'d, Blocking, N>` directly

## [0.3.0] - 2026-03-10

### Added

- `ferriswheel`: five new ring effects — `BreatheEffect`, `MeteorEffect`, `TwinkleEffect`, `FireEffect`, `CylonEffect` — plus `sine_full()` utility and `RGB8` re-export; `ferriswheel` now provides twelve effects in total
- `rustyfarian-esp-hal-ws2812`: full bare-metal WS2812 RMT driver (`esp-hal 1.0.0`, `no_std`); targets ESP32-C3, ESP32-C6, and ESP32-WROOM-32
- Both drivers implement `SmartLedsWrite` from `smart-leds-trait`, enabling `brightness()` and `gamma()` iterator adapters from the `smart-leds` ecosystem
- 19 ready-to-run examples across ESP32-C3, ESP32-C6, and ESP32-WROOM-32 for both drivers; `just run <example>` and `just build-example <crate> <example>` automation

### Changed

- `ferriswheel`: `FireEffect` gradient top stop changed from white to bright yellow; flames stay in the red → orange → yellow family

### Fixed

- `ferriswheel`: `BreatheEffect` and `PulseEffect` clamp via `u8::min`/`u8::max` before range arithmetic, preventing underflow when `min_brightness > max_brightness`
- `ferriswheel`: `PulseEffect::set_color` rustdoc renamed "breathing cycle" to "pulse cycle"
- Flashing reliability: `run-example` always uses the correct v5.3.3 bootloader; bare-metal examples now include `esp_app_desc!()` to prevent boot loop; `sdkconfig.defaults` raises the main task stack to 8 KB to prevent stack overflow with 12+ LEDs in debug builds

## [0.2.0] - 2026-03-01

### Fixed

- `PulseEffect::new()` doc comment now correctly states the default `min_brightness` is `2`, not `0`
- `SpinnerEffect` tail brightness calculation now uses `u16` arithmetic with `.max(1)` instead of a separate zero-floor branch, removing a redundant `let mut`

### Added

- `NoLed` stub in `led-effects`: a zero-size `StatusLed` implementor with `type Error = Infallible` for use when no physical LED is present
- `RainbowEffect::with_hue_offset(u8)` builder for setting the initial hue offset
- `RainbowEffect::set_hue_offset(&mut self, u8)` for live hue adjustment without resetting the rotation cycle
- `PulseEffect::set_color(&mut self, RGB8)` for changing color without resetting the breathing phase
- `SpinnerEffect::set_color(&mut self, RGB8)` for changing color without resetting the spinner position
- `ChaseEffect::set_color(&mut self, RGB8)` for changing color without resetting the chase position
- `FlashEffect::set_color(&mut self, RGB8)` for changing color without resetting the duty-cycle counter
