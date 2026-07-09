# pennant

[![crates.io version](https://img.shields.io/crates/v/pennant.svg)](https://crates.io/crates/pennant)
[![docs.rs docs](https://img.shields.io/docsrs/pennant)](https://docs.rs/pennant)

Status-LED traits and effects for embedded Rust — fully `no_std`.

Provides `StatusLed` and `AsyncStatusLed` traits that decouple application
code from concrete LED drivers, plus a `PulseEffect` for smooth pulsing
brightness animations and a `NoLed` zero-size stub.

The `hal` feature unlocks GPIO adapters built on
[`embedded-hal`](https://crates.io/crates/embedded-hal) 1.0:

- `SimpleLed` maps RGB colours onto a single plain on/off GPIO pin.
- `RgbGpioLed` drives a discrete (non-WS2812) RGB LED over three separate GPIOs,
  switching each channel on/off with a `Polarity` flag for common-anode
  (active-low) wiring — e.g. the Cheap Yellow Display's onboard RGB LED.
  Each channel is on/off only, so it renders **eight colours** (on/off per
  channel), not analog colour mixing — for smooth colours use `RgbPwmLed`.
- `RgbPwmLed` drives a discrete RGB LED over three `embedded-hal` `SetDutyCycle`
  PWM channels, mapping each colour component onto a duty cycle for **smooth
  analog colour mixing** and true brightness control. Shares the same `Polarity`
  flag as `RgbGpioLed` for common-anode / common-cathode wiring.

```toml
pennant = { version = "0.6", features = ["hal"] }
```

Part of the [`rustyfarian-ws2812`](https://github.com/datenkollektiv/rustyfarian-ws2812)
workspace alongside [`ferriswheel`](https://crates.io/crates/ferriswheel)
(ring effects) and [`bunting`](https://crates.io/crates/bunting) (WS2812 colour utilities).

## Example

```rust
use pennant::{NoLed, StatusLed};
use rgb::RGB8;

let mut led = NoLed::default();
led.set_color(RGB8::new(255, 0, 0)).unwrap(); // always Ok
```

## Documentation

Full API docs at [docs.rs/pennant](https://docs.rs/pennant).

## License

Dual-licensed under MIT or Apache-2.0.

## Changelog

See the [workspace CHANGELOG](https://github.com/datenkollektiv/rustyfarian-ws2812/blob/main/CHANGELOG.md)
for release notes across all crates.
