# rustyfarian-esp-hal-ws2812

[![crates.io version](https://img.shields.io/crates/v/rustyfarian-esp-hal-ws2812.svg)](https://crates.io/crates/rustyfarian-esp-hal-ws2812)
[![docs.rs docs](https://img.shields.io/docsrs/rustyfarian-esp-hal-ws2812)](https://docs.rs/rustyfarian-esp-hal-ws2812)

Bare-metal `no_std` WS2812 (NeoPixel) LED driver using the `esp-hal` RMT peripheral.

Drives WS2812/NeoPixel addressable LEDs on ESP32-C3, ESP32-C6, and classic
ESP32 targets without an OS or ESP-IDF. The driver is const-generic over the
pulse-code buffer size, allocates nothing at runtime, and supports both
blocking and `async` (Embassy) operation. Implements `SmartLedsWrite` for
the `smart-leds` ecosystem. For std/ESP-IDF projects see
`rustyfarian-esp-idf-ws2812`.

Part of the [`rustyfarian-ws2812`](https://github.com/datenkollektiv/rustyfarian-ws2812)
workspace alongside [`rustyfarian-esp-idf-ws2812`](https://crates.io/crates/rustyfarian-esp-idf-ws2812)
(std, ESP-IDF), [`rustyfarian-avr-ws2812`](https://crates.io/crates/rustyfarian-avr-ws2812)
(AVR), and the pure-logic crates [`ferriswheel`](https://crates.io/crates/ferriswheel),
[`pennant`](https://crates.io/crates/pennant), and [`bunting`](https://crates.io/crates/bunting).

## Example

The default feature enables `esp32c6`. To target a different chip, disable
defaults and select exactly one chip feature.

**ESP32-C6** (default — no `Cargo.toml` override needed):

```toml
rustyfarian-esp-hal-ws2812 = { version = "0.6" }
```

**ESP32-C3:**

```toml
rustyfarian-esp-hal-ws2812 = { version = "0.6", default-features = false, features = ["esp32c3", "unstable", "pennant"] }
```

**Classic ESP32 / WROOM-32** (requires the `esp` Xtensa toolchain):

```toml
rustyfarian-esp-hal-ws2812 = { version = "0.6", default-features = false, features = ["esp32", "unstable", "pennant"] }
```

```rust
use esp_hal::{rmt::{Rmt, TxChannelConfig, TxChannelCreator}, time::Rate};
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{Ws2812Rmt, buffer_size, RMT_CLK_DIV};

const NUM_LEDS: usize = 12;
const N: usize = buffer_size(NUM_LEDS);

let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
let config = TxChannelConfig::default()
    .with_clk_divider(RMT_CLK_DIV)
    .with_idle_output(true);
let channel = rmt.channel0.configure_tx(&config).unwrap().with_pin(peripherals.GPIO18);

let mut led: Ws2812Rmt<_, N> = Ws2812Rmt::new(channel);
led.set_pixel(RGB8::new(255, 0, 0)).unwrap();
```

## Supported Targets

| Feature             | Target triple                  | Boards                                      |
|:--------------------|:-------------------------------|:--------------------------------------------|
| `esp32c6` (default) | `riscv32imac-unknown-none-elf` | ESP32-C6-DevKitC-1                          |
| `esp32c3`           | `riscv32imc-unknown-none-elf`  | ESP32-C3-DevKit-Rust-1, ESP32-C3-DevKitC-02 |
| `esp32`             | `xtensa-esp32-none-elf`        | ESP32-WROOM-32 (requires `esp` toolchain)   |

## Features

| Feature       | Default  | Description                                                                     |
|:--------------|:---------|:--------------------------------------------------------------------------------|
| `esp32c6`     | yes      | chip support for ESP32-C6                                                       |
| `esp32c3`     | no       | chip support for ESP32-C3                                                       |
| `esp32`       | no       | chip support for classic ESP32 / WROOM-32                                       |
| `async`       | no       | async `Ws2812Rmt` via Embassy executor                                          |
| `pennant`     | yes      | implements `StatusLed`; `AsyncStatusLed` when `async` is also enabled           |
| `smart-leds`  | no       | enables `smart-leds` brightness and gamma adapters                              |
| `rt`          | no       | enables `esp-hal` runtime (for `#[entry]` examples)                             |
| `esp-println` | no       | enables `esp-println` for debug output in examples                              |

## Documentation

Full API docs at [docs.rs/rustyfarian-esp-hal-ws2812](https://docs.rs/rustyfarian-esp-hal-ws2812).

## License

Dual-licensed under MIT or Apache-2.0.

## Changelog

See the [workspace CHANGELOG](https://github.com/datenkollektiv/rustyfarian-ws2812/blob/main/CHANGELOG.md)
for release notes across all crates.
