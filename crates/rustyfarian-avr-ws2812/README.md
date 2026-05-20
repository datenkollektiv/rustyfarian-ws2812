# rustyfarian-avr-ws2812

[![crates.io version](https://img.shields.io/crates/v/rustyfarian-avr-ws2812.svg)](https://crates.io/crates/rustyfarian-avr-ws2812)
[![docs.rs docs](https://img.shields.io/docsrs/rustyfarian-avr-ws2812)](https://docs.rs/rustyfarian-avr-ws2812)

`no_std` WS2812 (NeoPixel) LED driver for AVR, with two hardware backends.

Targets ATmega328P (Arduino Uno/Nano) and shares the same `&[RGB8]`-based
public API as the ESP32 driver crates in this workspace — every
[`ferriswheel`](https://crates.io/crates/ferriswheel) effect runs unchanged.

Part of the [`rustyfarian-ws2812`](https://github.com/datenkollektiv/rustyfarian-ws2812)
workspace alongside [`rustyfarian-esp-hal-ws2812`](https://crates.io/crates/rustyfarian-esp-hal-ws2812)
(bare-metal ESP32), [`rustyfarian-esp-idf-ws2812`](https://crates.io/crates/rustyfarian-esp-idf-ws2812)
(std, ESP-IDF), and the pure-logic crates [`ferriswheel`](https://crates.io/crates/ferriswheel),
[`pennant`](https://crates.io/crates/pennant), and [`bunting`](https://crates.io/crates/bunting).

## Choosing a Backend

|               | `Ws2812Spi` — SPI prerendered | `Ws2812BitBang` — cycle-counted asm |
|:--------------|:------------------------------|:------------------------------------|
| Cargo feature | always available              | `bitbang`                           |
| Hardware      | any AVR with SPI              | ATmega328P @ 16 MHz, any GPIO       |
| Status        | works on tolerant strips      | **recommended; hardware-validated** |
| Interrupts    | caller disables               | disabled internally                 |

The bit-bang backend is recommended. Hardware testing showed the SPI
prerendered encoding's out-of-spec `T1H` causes stable white-ish output and
chain leakage on both genuine and clone Arduino Nanos, while the bit-bang
backend renders correctly on the same hardware (see [ADR 007](https://github.com/datenkollektiv/rustyfarian-ws2812/blob/main/docs/adr/007-avr-ws2812-driver-strategy.md)).

## Example (bit-bang backend)

Add to `Cargo.toml`:

```toml
rustyfarian-avr-ws2812 = { version = "0.6", features = ["bitbang"] }
```

Requires nightly Rust for AVR — pin the toolchain in `rust-toolchain.toml`
and build with `-Z build-std=core --target avr-none`:

The type is const-generic over port address and pin bit.
`0x0B` is `PORTD` on ATmega328P; bit `2` maps to Arduino pin D2.

```rust
use rustyfarian_avr_ws2812::Ws2812BitBang;
use rgb::RGB8;

let mut ws: Ws2812BitBang<_, 0x0B, 2> = Ws2812BitBang::new(pin);
let colors = [RGB8::new(255, 0, 0); 8];
ws.write(&colors).unwrap();
```

## Features

| Feature            | Default  | Description                                                 |
|:-------------------|:---------|:------------------------------------------------------------|
| `bitbang`          | no       | cycle-counted inline-asm bit-bang backend (`Ws2812BitBang`) |
| `smart-leds-trait` | no       | `SmartLedsWrite` impl for both backends                     |

## Documentation

Full API docs at [docs.rs/rustyfarian-avr-ws2812](https://docs.rs/rustyfarian-avr-ws2812).

## License

Dual-licensed under MIT or Apache-2.0.

## Changelog

See the [workspace CHANGELOG](https://github.com/datenkollektiv/rustyfarian-ws2812/blob/main/CHANGELOG.md)
for release notes across all crates.
