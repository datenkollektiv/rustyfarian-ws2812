# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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
