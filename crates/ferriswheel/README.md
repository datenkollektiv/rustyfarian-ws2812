# ferriswheel

[![crates.io version](https://img.shields.io/crates/v/ferriswheel.svg)](https://crates.io/crates/ferriswheel)
[![docs.rs docs](https://img.shields.io/docsrs/ferriswheel)](https://docs.rs/ferriswheel)

RGB LED ring effects and animations — fully `no_std`, fully unit-testable.

Fourteen ring-specific effects (rainbow, pulse, breathe, spinner, meteor,
twinkle, fire, cylon, knight rider, chase, flash, progress, section, rainbow
comet) all behind a single `Effect` trait that renders into an `&mut [RGB8]`
buffer. No hardware dependency — drop in any WS2812 driver, or use one of the
sister driver crates from the workspace.

Part of the [`rustyfarian-ws2812`](https://github.com/datenkollektiv/rustyfarian-ws2812)
workspace alongside [`pennant`](https://crates.io/crates/pennant)
(status-LED adapters) and [`bunting`](https://crates.io/crates/bunting)
(WS2812 colour utilities).

## Example

```rust
use ferriswheel::{Effect, RainbowEffect, RGB8};

let mut rainbow = RainbowEffect::new(12).unwrap();
let mut buffer = [RGB8::default(); 12];

rainbow.update(&mut buffer).unwrap();
```

## Documentation

Full API docs at [docs.rs/ferriswheel](https://docs.rs/ferriswheel).

## License

Dual-licensed under MIT or Apache-2.0.

## Changelog

See the [workspace CHANGELOG](https://github.com/datenkollektiv/rustyfarian-ws2812/blob/main/CHANGELOG.md)
for release notes across all crates.
