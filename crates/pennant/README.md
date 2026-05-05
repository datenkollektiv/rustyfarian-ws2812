# pennant

Status-LED traits and effects for embedded Rust — fully `no_std`.

Provides `StatusLed` and `AsyncStatusLed` traits that decouple application
code from concrete LED drivers, plus a `PulseEffect` for smooth pulsing
brightness animations and a `NoLed` zero-size stub.

The `hal` feature unlocks an additional `SimpleLed` adapter that maps RGB
colours onto plain on/off GPIO pins via
[`embedded-hal`](https://crates.io/crates/embedded-hal) 1.0:

```toml
pennant = { version = "0.5", features = ["hal"] }
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
