# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed

- `rustyfarian-esp-hal-ws2812`: chip and `unstable` feature selection moved from the workspace root into the driver crate's own `[features]` (`esp32c6`, `unstable`);
  the workspace only pins the version now, making the crate self-describing and easier to extend for future chips

### Added

- `rustyfarian-esp-hal-ws2812`: full WS2812 RMT driver implementation for ESP32-C6 using `esp-hal 1.0.0`
  (bare-metal, `no_std`); const-generic buffer size `N = num_leds * 24 + 1`
- `buffer_size(num_leds)` const helper in `rustyfarian-esp-hal-ws2812` to compute the correct `N` at compile time
- `RMT_CLK_DIV` constant (`8`) exported from `rustyfarian-esp-hal-ws2812` for correct 10 MHz RMT clock configuration
- `just check-hal` recipe: checks the bare-metal crate without the ESP-IDF toolchain
  (requires `rustup target add riscv32imac-unknown-none-elf`)
- `just clippy-hal` recipe: runs clippy with `-D warnings` on the bare-metal crate

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
