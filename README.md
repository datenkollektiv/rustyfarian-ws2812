# Rustyfarian WS2812 Related Crates

[![CI](https://github.com/datenkollektiv/rustyfarian-ws2812/actions/workflows/rust.yml/badge.svg)](https://github.com/datenkollektiv/rustyfarian-ws2812/actions/workflows/rust.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)
[![cargo fmt](https://github.com/datenkollektiv/rustyfarian-ws2812/actions/workflows/fmt.yml/badge.svg)](https://github.com/datenkollektiv/rustyfarian-ws2812/actions/workflows/fmt.yml)
[![cargo clippy](https://github.com/datenkollektiv/rustyfarian-ws2812/actions/workflows/clippy.yml/badge.svg)](https://github.com/datenkollektiv/rustyfarian-ws2812/actions/workflows/clippy.yml)
[![cargo audit](https://github.com/datenkollektiv/rustyfarian-ws2812/actions/workflows/audit.yml/badge.svg)](https://github.com/datenkollektiv/rustyfarian-ws2812/actions/workflows/audit.yml)

Library-only workspace providing WS2812 (NeoPixel) LED support for ESP32 and `no_std` embedded Rust projects.
No application code — just reusable, composable crates.

TL;DR Grab an ESP32-C6 board and check it out:

```sh
just run idf_c6_effects
```

## Vision

> Provide reusable, sans-io WS2812 LED crates for embedded Rust developers —
> pure logic first, hardware wrappers thin, everything testable without hardware.

**We are building this for:** Embedded Rust developers building WS2812-based LED projects on ESP32 who want testable, composable building blocks rather than monolithic driver crates.

**Long-term goals:**
- Animation vocabulary on demand — users find what they need without forking
- Complete `no_std` / embassy support via `rustyfarian-esp-hal-ws2812`
- Ecosystem currency — timely adoption of new ESP32 chip variants and HAL updates

**Out of scope:** Application code, exhaustive pre-built animation catalogues, and anything that doesn't serve the embedded WS2812 use case.

*Full vision, success signals, and open questions: [VISION.md](./VISION.md)*

## Rustyfarian Philosophy

This library embodies the principle of **extracting testable pure logic from hardware-specific code**—a pattern common in application development but rare in embedded Rust.

- Pure functions belong in `no_std` crates (`ws2812-pure`, `led-effects`, `ferriswheel`)
- Hardware-specific wrappers should be thin, delegating logic to pure functions
- If you can unit test it without hardware, it should be in a testable crate
- Ring-specific animations live in `ferriswheel` so they can be reused and tested independently

The radical separation into multiple crates means `ws2812-pure` (color conversion logic) and `ferriswheel` (ring animations) can be fully unit-tested on your laptop without an ESP32 or ESP toolchain.
Most embedded LED libraries require a device to verify even pure logic.

See [Why Yet Another WS2812 Crate?](docs/why-yet-another-ws2812-crate.md) for the full design rationale.

> Note: Large parts of this library (and documentation) were developed with the assistance of AI tools.
> All generated code has been reviewed and curated by the maintainer.

## Crates

| Crate                                                             | Description                                                                                                                                             | Target               |
|:------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------|:---------------------|
| [`ferriswheel`](crates/ferriswheel)                               | RGB LED ring animations (rainbow, pulse, breathe, spinner, meteor, twinkle, fire, cylon, knight rider, chase, flash, progress, sections, rainbow comet) | `no_std` compatible  |
| [`led-effects`](crates/led-effects)                               | LED status effects (pulse, simple LED adapter)                                                                                                          | `no_std` compatible  |
| [`ws2812-pure`](crates/ws2812-pure)                               | Pure Rust WS2812 utilities (color conversion, bit encoding)                                                                                             | `no_std` compatible  |
| [`rustyfarian-esp-idf-ws2812`](crates/rustyfarian-esp-idf-ws2812) | WS2812 driver using ESP-IDF RMT peripheral                                                                                                              | ESP-IDF (std)        |
| [`rustyfarian-esp-hal-ws2812`](crates/rustyfarian-esp-hal-ws2812) | WS2812 driver using esp-hal RMT peripheral                                                                                                              | esp-hal (no_std)     |
| [`rustyfarian-avr-ws2812`](crates/rustyfarian-avr-ws2812)         | WS2812 driver using SPI prerendered encoding (`embedded-hal` 1.0)                                                                                       | AVR (no_std)         |

## Examples

The project includes ready-to-run examples for ESP32-C3 and ESP32-C6 using both drivers.
Each example name encodes the driver, chip, and effect: `{driver}_{chip}_{effect}`.

| Example                      | Driver           | Board / Chip              | Effect                 | Data GPIO  | Notes                   |
|:-----------------------------|:-----------------|:--------------------------|:-----------------------|:-----------|:------------------------|
| `hal_c3_pulse`               | esp-hal (no_std) | ESP32-C3                  | Pulse                  | GPIO4      |                         |
| `hal_c6_pulse`               | esp-hal (no_std) | ESP32-C6                  | Pulse                  | GPIO18     |                         |
| `hal_esp32_pulse`            | esp-hal (no_std) | ESP32-WROOM-32            | Pulse                  | GPIO4      |                         |
| `hal_c6_breathe_color`       | esp-hal (no_std) | ESP32-C6                  | Breathe (hue cycling)  | GPIO18     |                         |
| `hal_c6_meteor`              | esp-hal (no_std) | ESP32-C6                  | Meteor / comet         | GPIO18     |                         |
| `hal_c6_twinkle`             | esp-hal (no_std) | ESP32-C6                  | Twinkle / starfield    | GPIO18     |                         |
| `hal_c6_fire`                | esp-hal (no_std) | ESP32-C6                  | Fire                   | GPIO18     |                         |
| `hal_c6_cylon`               | esp-hal (no_std) | ESP32-C6                  | Cylon / scanner        | GPIO18     |                         |
| `hal_c6_knight_rider`        | esp-hal (no_std) | ESP32-C6                  | Knight Rider / scanner | GPIO18     |                         |
| `hal_c6_rainbow_comet`       | esp-hal (no_std) | ESP32-C6                  | Rainbow comet          | GPIO18     |                         |
| `hal_c6_rainbow_comet_async` | esp-hal (no_std) | ESP32-C6                  | Rainbow comet (async)  | GPIO18     | `--features async`      |
| `hal_c3_pulse_async`         | esp-hal (no_std) | ESP32-C3                  | Pulse (async)          | GPIO4      | `--features async`      |
| `hal_c6_pulse_async`         | esp-hal (no_std) | ESP32-C6                  | Pulse (async)          | GPIO18     | `--features async`      |
| `hal_esp32_pulse_async`      | esp-hal (no_std) | ESP32-WROOM-32            | Pulse (async)          | GPIO4      | `--features async`      |
| `hal_c6_smart_leds`          | esp-hal (no_std) | ESP32-C6                  | smart-leds interop     | GPIO18     | `--features smart-leds` |
| `idf_c3_rainbow`             | ESP-IDF (std)    | ESP32-C3                  | Rainbow                | GPIO4      |                         |
| `idf_c6_rainbow`             | ESP-IDF (std)    | ESP32-C6                  | Rainbow                | GPIO18     |                         |
| `idf_esp32_rainbow`          | ESP-IDF (std)    | Adafruit Feather ESP32 V2 | Rainbow                | GPIO0      | GPIO2 = power enable    |
| `idf_c6_breathe_color`       | ESP-IDF (std)    | ESP32-C6                  | Breathe (hue cycling)  | GPIO18     |                         |
| `idf_c6_effects`             | ESP-IDF (std)    | ESP32-C6                  | Multi-effect sequence  | GPIO18     |                         |
| `idf_c6_meteor`              | ESP-IDF (std)    | ESP32-C6                  | Meteor / comet         | GPIO18     |                         |
| `idf_c6_twinkle`             | ESP-IDF (std)    | ESP32-C6                  | Twinkle / starfield    | GPIO18     |                         |
| `idf_c6_fire`                | ESP-IDF (std)    | ESP32-C6                  | Fire                   | GPIO18     |                         |
| `idf_c6_cylon`               | ESP-IDF (std)    | ESP32-C6                  | Cylon / scanner        | GPIO18     |                         |
| `idf_c6_knight_rider`        | ESP-IDF (std)    | ESP32-C6                  | Knight Rider / scanner | GPIO18     |                         |
| `idf_c6_rainbow_comet`       | ESP-IDF (std)    | ESP32-C6                  | Rainbow comet          | GPIO18     |                         |
| `idf_c6_smart_leds`          | ESP-IDF (std)    | ESP32-C6                  | smart-leds interop     | GPIO18     | `--features smart-leds` |
| `avr-nano-rainbow`           | AVR SPI (no_std) | Arduino Nano (ATmega328P) | Rainbow                | D11 (MOSI) | Standalone project      |

Flash and open the serial monitor on a connected board:

```sh
just run hal_c6_pulse
```

Build only (no board required):

```sh
just build-example hal-ws2812 hal_c6_pulse
just build-example idf-ws2812 idf_c3_rainbow
```

HAL examples (`hal_*`) require the bare-metal target:

```sh
rustup target add riscv32imac-unknown-none-elf
```

IDF examples (`idf_*`) require `cargo +esp` (install via `espup`).
The Feather ESP32 V2 examples additionally use the Xtensa toolchain (`+esp`) with target `xtensa-esp32-espidf`.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
rustyfarian-esp-idf-ws2812 = { git = "https://github.com/datenkollektiv/rustyfarian-ws2812" }
```

For `no_std` projects that only need the pure utilities:

```toml
[dependencies]
ferriswheel = { git = "https://github.com/datenkollektiv/rustyfarian-ws2812" }
led-effects = { git = "https://github.com/datenkollektiv/rustyfarian-ws2812", default-features = false }
ws2812-pure = { git = "https://github.com/datenkollektiv/rustyfarian-ws2812" }
```

## Example

```rust
use rustyfarian_esp_idf_ws2812::WS2812RMT;
use led_effects::PulseEffect;
use rgb::RGB8;

// Initialize driver
let mut driver = WS2812RMT::new(gpio_pin)?;

// Set a single pixel
driver.set_pixel(RGB8::new(255, 0, 0))?;

// Use pulse animation
let mut pulse = PulseEffect::new();
loop {
    let color = pulse.update((0, 0, 255));
    driver.set_pixel(color)?;
    // delay...
}
```

### Rainbow Effect

For LED rings, use `RainbowEffect` from the `ferriswheel` crate:

```rust
use rustyfarian_esp_idf_ws2812::WS2812RMT;
use ferriswheel::{RainbowEffect, Direction, RGB8};

let mut driver = WS2812RMT::new(gpio_pin)?;

let mut rainbow = RainbowEffect::new(12)?
    .with_speed(2)?
    .with_brightness(128)
    .with_direction(Direction::Clockwise);

let mut buffer = [RGB8::default(); 12];

loop {
    rainbow.update(&mut buffer)?;
    driver.set_pixels_slice(&buffer)?;
    // delay...
}
```

## Development

A [`justfile`](justfile) provides all common development tasks.
The workspace defaults to the ESP32 target, so `just` recipes override the target automatically for platform-independent crates.

List available recipes:

```sh
just
```

Common workflows:

```sh
just verify
```

```sh
just pre-commit
```

```sh
just ci
```

### IDF Troubleshooting

If `sdkconfig.defaults` changes have no effect after a rebuild, `esp-idf-sys` may not have re-run its build script.
Clear the stale cache with:

```sh
just clean-idf
```

Then rebuild the IDF example to repopulate build artifacts before flashing.

## License

MIT or Apache-2.0
