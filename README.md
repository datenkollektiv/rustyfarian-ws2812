# Rustyfarian WS2812 Related Crates

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
esp32-ws2812-rmt = { git = "https://github.com/datenkollektiv/esp32-ws2812" }
```

For `no_std` projects that only need the core utilities:

```toml
[dependencies]
led-effects = { git = "https://github.com/datenkollektiv/esp32-ws2812", default-features = false }
ws2812-core = { git = "https://github.com/datenkollektiv/esp32-ws2812", default-features = false }
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

## License

MIT or Apache-2.0
