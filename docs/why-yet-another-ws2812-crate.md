# Why Yet Another WS2812 Crate?

This library embodies the principle of "extract testable pure logic from hardware-specific code"—a pattern inspired by [sans-io](https://sans-io.readthedocs.io/) that's common in application development but rare in embedded Rust.
Your RGB clock benefits from battle-tested color math without needing an ESP32 plugged in during development.

## Testability Without Hardware

The radical separation into three crates means `bunting` (color conversion logic) can be fully unit-tested on your machine without an ESP32 or ESP toolchain.
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
This makes your code testable (e.g., mock LEDs for tests, real WS2812s for production).

## `no_std` at the Core

`bunting` and `led-effects` are fully `no_std` with zero allocations.
The ESP-specific parts only exist in `rustyfarian-esp-idf-ws2812`.
This separation is unusual among LED crates and is significant because it keeps the core logic portable, easily testable on a desktop, and usable on bare-metal targets while confining platform-specific code to a single crate.
Most LED crates assume `std` throughout.

## Zero Dynamic Allocation in the Driver

The RMT driver uses fixed-size stack arrays (`[Pulse; 48]`) instead of `Vec`, avoiding heap fragmentation in long-running embedded applications.

## Library-Only Philosophy

No example apps or binaries.
Just composable, reusable crates.
Your downstream RGB project can consume these as building blocks rather than forking/copying code.

## Optional Feature Coupling

The `led-effects` integration is behind a feature flag, so minimal projects can skip the abstraction layer entirely.

## Distinction from Spatial LED Frameworks

[`blinksy`](https://crates.io/crates/blinksy) is a separate, spatially-oriented Rust LED framework targeting 1D, 2D, and (planned) 3D LED *installations* — panels, art pieces, lighting controllers.
It uses a stateless `Pattern<Dim, Layout>` model that computes colours from coordinates and elapsed time.

This is genuinely complementary to `ferriswheel`, not redundant.
Three things `ferriswheel` provides that `blinksy` does not:

- **Host-runnable unit tests for every effect.**
  `ferriswheel` ships 316 unit tests that run on a laptop via `cargo test --target <host-triple>`, with no hardware in the loop.
  `blinksy` ships visual simulation (`blinksy-desktop`) but no unit tests.
- **A ring-specific effect vocabulary.**
  More than a dozen effects (rainbow, pulse, breathe, spinner, meteor, twinkle, fire, cylon, knight rider, chase, flash, progress, sections, rainbow comet) all designed with the topology of a ring in mind, behind a stateful `Effect` trait.
  `blinksy` ships two patterns (Rainbow, Noise) and represents a ring as a special case of a 2D arc.
- **MIT / Apache-2.0 dual licensing.**
  `blinksy` is licensed under EUPL-1.2, which is incompatible with permissive licences as a Cargo dependency in MIT/Apache-2.0 projects.

Choose `blinksy` for spatial lighting installations and panel-style displays.
Choose `ferriswheel` for embedded rings with a small, testable effect loop.

A full comparison is in [docs/blinksy-ecosystem-evaluation.md](blinksy-ecosystem-evaluation.md).
