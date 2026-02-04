# Roadmap for Rustyfarian ws2812 Crate

*Last Updated: February 2026*

## Overview

This roadmap tracks the development progress of the rustyfarian-ws2812 crate.
It outlines features that might be implemented in future versions, if any.

### Implement NoLed

```rust
/// Dummy LED type — an app might not want a status LED, but WiFiManager (rustyfarian-network) requires the type parameter.
struct NoLed;

impl StatusLed for NoLed {
    type Error = std::convert::Infallible;
    fn set_color(&mut self, _color: rgb::RGB8) -> Result<(), Self::Error> {
        Ok(())
    }
}
```
