# rustyfarian-esp-idf-ws2812

[![crates.io version](https://img.shields.io/crates/v/rustyfarian-esp-idf-ws2812.svg)](https://crates.io/crates/rustyfarian-esp-idf-ws2812)
[![docs.rs docs](https://img.shields.io/docsrs/rustyfarian-esp-idf-ws2812)](https://docs.rs/rustyfarian-esp-idf-ws2812)

WS2812 (NeoPixel) LED driver using the ESP-IDF RMT peripheral.

Drives WS2812/NeoPixel addressable LEDs on any ESP32 variant that supports
RMT via ESP-IDF. Provides `set_pixel` and `set_pixels_slice` for direct use,
and implements `SmartLedsWrite` for the `smart-leds` ecosystem.
Uses `anyhow::Result` for error propagation. Blocking-only; for bare-metal
`async` support see `rustyfarian-esp-hal-ws2812`.

Part of the [`rustyfarian-ws2812`](https://github.com/datenkollektiv/rustyfarian-ws2812)
workspace alongside [`rustyfarian-esp-hal-ws2812`](https://crates.io/crates/rustyfarian-esp-hal-ws2812)
(bare-metal, `no_std`), [`rustyfarian-avr-ws2812`](https://crates.io/crates/rustyfarian-avr-ws2812)
(AVR), and the pure-logic crates [`ferriswheel`](https://crates.io/crates/ferriswheel),
[`pennant`](https://crates.io/crates/pennant), and [`bunting`](https://crates.io/crates/bunting).

## Example

```rust
use rustyfarian_esp_idf_ws2812::Ws2812Rmt;
use rgb::RGB8;

let mut led = Ws2812Rmt::new(peripherals.pins.gpio8)?;

led.set_pixel(RGB8::new(255, 0, 0))?;

let colors = [RGB8::new(255, 0, 0), RGB8::new(0, 255, 0), RGB8::new(0, 0, 255)];
led.set_pixels_slice(&colors)?;
```

## Supported Boards

Any ESP32 variant with RMT support via ESP-IDF.
`Ws2812Rmt::new()` uses a default channel configuration tuned for ESP32-C3/C6
(48-symbol RMT blocks, 2 TX channels).
For classic ESP32, S2, or S3 use [`new_with_channel_config`](https://docs.rs/rustyfarian-esp-idf-ws2812)
with `memory_block_symbols: 64`.

| Board                    | GPIO                 |
|:-------------------------|:---------------------|
| ESP32-C3-DevKit-Rust-1   | GPIO2                |
| ESP32-C3-DevKitC-02      | GPIO8                |
| ESP32-C6-DevKitC-1       | GPIO8                |
| Classic ESP32 / WROOM-32 | any RMT-capable GPIO |

## Features

| Feature      | Default  | Description                                                |
|:-------------|:---------|:-----------------------------------------------------------|
| `pennant`    | yes      | implements [`pennant::StatusLed`](https://docs.rs/pennant) |
| `smart-leds` | no       | enables `smart-leds` brightness and gamma adapters         |

## Documentation

Full API docs at [docs.rs/rustyfarian-esp-idf-ws2812](https://docs.rs/rustyfarian-esp-idf-ws2812).

## License

Dual-licensed under MIT or Apache-2.0.

## Changelog

See the [workspace CHANGELOG](https://github.com/datenkollektiv/rustyfarian-ws2812/blob/main/CHANGELOG.md)
for release notes across all crates.
