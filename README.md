# Rustyfarian WS2812 Related Crates

[![CI](https://github.com/datenkollektiv/rustyfarian-ws2812/actions/workflows/rust.yml/badge.svg)](https://github.com/datenkollektiv/rustyfarian-ws2812/actions/workflows/rust.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)
[![Last Commit](https://img.shields.io/github/last-commit/datenkollektiv/rustyfarian-ws2812)](https://github.com/datenkollektiv/rustyfarian-ws2812/commits/)

Library-only workspace providing WS2812 (NeoPixel) LED support for ESP32 and `no_std` embedded Rust projects.
No application code—just reusable, composable crates.

## Philosophy

This library embodies the principle of **extracting testable pure logic from hardware-specific code**—a pattern common in application development but rare in embedded Rust.

- Pure functions belong in `no_std` crates (`ws2812-pure`, `led-effects`, `ferriswheel`)
- Hardware-specific wrappers should be thin, delegating logic to pure functions
- If you can unit test it without hardware, it should be in a testable crate
- Ring-specific animations live in `ferriswheel` so they can be reused and tested independently

The radical separation into multiple crates means `ws2812-pure` (color conversion logic) and `ferriswheel` (ring animations) can be fully unit-tested on your laptop without an ESP32 or ESP toolchain.
Most embedded LED libraries require a device to verify even pure logic.

See [Why Yet Another WS2812 Crate?](docs/why-yet-another-ws2812-crate.md) for the full design rationale.

> Note: Parts of this library were developed with the assistance of AI tools.
> All generated code has been reviewed and curated by the maintainer.

## Crates

| Crate                                         | Description                                                 | Target              |
|:----------------------------------------------|-------------------------------------------------------------|:--------------------|
| [`ferriswheel`](crates/ferriswheel)           | RGB LED ring animations (rainbow, HSV utilities)            | `no_std` compatible |
| [`led-effects`](crates/led-effects)           | LED status effects (pulse, simple LED adapter)              | `no_std` compatible |
| [`ws2812-pure`](crates/ws2812-pure)           | Pure Rust WS2812 utilities (color conversion, bit encoding) | `no_std` compatible |
| [`rustyfarian-esp-idf-ws2812`](crates/rustyfarian-esp-idf-ws2812) | WS2812 driver using ESP-IDF RMT peripheral                  | ESP-IDF (std)       |
| [`rustyfarian-esp-hal-ws2812`](crates/rustyfarian-esp-hal-ws2812) | WS2812 driver using esp-hal RMT peripheral (skeleton)       | esp-hal (no_std)    |

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
let mut driver = WS2812RMT::new(gpio_pin, rmt_channel)?;

// Set a single pixel
driver.set_pixel(0, RGB8::new(255, 0, 0))?;

// Use pulse animation
let mut pulse = PulseEffect::new();
loop {
    let color = pulse.update((0, 0, 255));
    driver.set_pixel(0, color)?;
    // delay...
}
```

### Rainbow Effect

For LED rings, use `RainbowEffect` from the `ferriswheel` crate:

```rust
use rustyfarian_esp_idf_ws2812::WS2812RMT;
use ferriswheel::{RainbowEffect, Direction};
use rgb::RGB8;

let mut driver = WS2812RMT::new(gpio_pin, rmt_channel)?;

let mut rainbow = RainbowEffect::new(12)?
    .with_speed(2)?
    .with_brightness(128)
    .with_direction(Direction::Clockwise);

let mut buffer = [RGB8::default(); 12];

loop {
    rainbow.update(&mut buffer)?;
    driver.set_pixels(&buffer)?;
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
just ci
```

See `CLAUDE.md` for the full testing and verification policy.

## License

MIT or Apache-2.0
