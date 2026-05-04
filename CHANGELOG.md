# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.5.0] - 2026-05-05

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
