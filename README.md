# Rustyfarian WS2812 Related Crates

[![CI](https://github.com/datenkollektiv/rustyfarian-ws2812/actions/workflows/rust.yml/badge.svg)](https://github.com/datenkollektiv/rustyfarian-ws2812/actions/workflows/rust.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.77%2B-orange.svg)](https://www.rust-lang.org)
[![Last Commit](https://img.shields.io/github/last-commit/datenkollektiv/rustyfarian-ws2812)](https://github.com/datenkollektiv/rustyfarian-ws2812/commits/)

WS2812 (NeoPixel) LED libraries for ESP32 and embedded Rust projects.

> Note: Parts of this library were developed with the assistance of AI tools.
> All generated code has been reviewed and curated by the maintainer.

## Crates

| Crate                                         | Description                                                 | Target              |
|:----------------------------------------------|-------------------------------------------------------------|:--------------------|
| [`led-effects`](crates/led-effects)           | Reusable LED animation effects (pulse, etc.)                | `no_std` compatible |
| [`ws2812-core`](crates/ws2812-core)           | Pure Rust WS2812 utilities (color conversion, bit encoding) | `no_std` compatible |
| [`esp32-ws2812-rmt`](crates/esp32-ws2812-rmt) | WS2812 driver using ESP32 RMT peripheral                    | ESP32 only          |

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
esp32-ws2812-rmt = { git = "https://github.com/datenkollektiv/rustyfarian-ws2812" }
```

For `no_std` projects that only need the core utilities:

```toml
[dependencies]
led-effects = { git = "https://github.com/datenkollektiv/rustyfarian-ws2812", default-features = false }
ws2812-core = { git = "https://github.com/datenkollektiv/rustyfarian-ws2812", default-features = false }
```

## Example

```rust
use esp32_ws2812_rmt::WS2812RMT;
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

For LED rings, use `RainbowEffect` to create smooth rainbow animations:

```rust
use esp32_ws2812_rmt::WS2812RMT;
use led_effects::{RainbowEffect, Direction};
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

## License

MIT or Apache-2.0
