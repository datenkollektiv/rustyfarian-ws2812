# Why Yet Another WS2812 Crate?

This library embodies the principle of "extract testable pure logic from hardware-specific code"—a pattern inspired by [sans-io](https://sans-io.readthedocs.io/) that's common in application development but rare in embedded Rust.
Your RGB clock benefits from battle-tested color math without needing an ESP32 plugged in during development.

## Testability Without Hardware

The radical separation into three crates means `ws2812-pure` (color conversion logic) can be fully unit-tested on your machine without an ESP32 or ESP toolchain.
Most embedded LED libraries require a device to verify even pure logic.

## The StatusLed Trait Abstraction

The `led-effects` crate provides a `StatusLed` trait that decouples your application from the LED implementation:

```rust
pub trait StatusLed {
    type Error;
    fn set_color(&mut self, color: RGB8) -> Result<(), Self::Error>;
}
```

Your RGB clock code can depend on this trait rather than a specific driver.
This makes your code testable (e.g., mock LEDs in tests, real WS2812s in production).

## `no_std` at the Core

`ws2812-pure` and `led-effects` are fully `no_std` with zero allocations.
The ESP-specific parts only exist in `rustyfarian-esp-idf-ws2812`.
This is unusual.
Most LED crates assume `std` throughout.

## Zero Dynamic Allocation in the Driver

The RMT driver uses fixed-size stack arrays (`[Pulse; 48]`) instead of `Vec`, avoiding heap fragmentation in long-running embedded applications.

## Library-Only Philosophy

No example apps or binaries.
Just composable, reusable crates.
Your downstream RGB project can consume these as building blocks rather than forking/copying code.

## Optional Feature Coupling

The `led-effects` integration is behind a feature flag, so minimal projects can skip the abstraction layer entirely.
