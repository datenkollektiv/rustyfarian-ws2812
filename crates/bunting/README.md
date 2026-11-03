# bunting

[![crates.io version](https://img.shields.io/crates/v/bunting.svg)](https://crates.io/crates/bunting)
[![docs.rs docs](https://img.shields.io/docsrs/bunting)](https://docs.rs/bunting)

Pure Rust WS2812 colour utilities — no hardware dependencies, fully `no_std`.

Encodes RGB pixel data into the bit and byte forms WS2812 hardware expects: GRB packing, MSB-first bit extraction and SPI-prerendered byte buffers.
It also owns the shared WS2812 pulse timings (`WS2812_T0H_NS` and friends, `WS2812_RESET_US`) with a `const fn ns_to_ticks` converter, and a `grid` module for addressing rectangular LED matrices.
All of it is hardware-independent and unit-testable on any host.

Part of the [`rustyfarian-ws2812`](https://github.com/datenkollektiv/rustyfarian-ws2812)
workspace alongside [`ferriswheel`](https://crates.io/crates/ferriswheel)
(ring effects) and [`pennant`](https://crates.io/crates/pennant) (status-LED adapters).

## Example

```rust
use bunting::rgb_to_grb;
use rgb::RGB8;

let red = RGB8::new(255, 0, 0);
assert_eq!(rgb_to_grb(red), 0x00FF00); // Green=0, Red=255, Blue=0
```

## Documentation

Full API docs at [docs.rs/bunting](https://docs.rs/bunting).

## License

Dual-licensed under MIT or Apache-2.0.

## Changelog

See the [workspace CHANGELOG](https://github.com/datenkollektiv/rustyfarian-ws2812/blob/main/CHANGELOG.md)
for release notes across all crates.
