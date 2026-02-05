# ADR 001: RGB Color Type Dependency

## Status

Accepted

## Context

The workspace currently depends on the external `rgb` crate (v0.8) for its `RGB8` color type.
This type is used across multiple crates:

- `ws2812-pure`: `rgb_to_grb(RGB8) -> u32`
- `ferriswheel`: `hsv_to_rgb() -> RGB8`, `RainbowEffect::update(&mut [RGB8])`
- `led-effects`: `StatusLed::set_color(RGB8)`, `PulseEffect::update() -> RGB8`
- `esp32-ws2812-rmt`: `set_pixel(RGB8)`, `set_pixels_slice(&[RGB8])`

Feedback from the first downstream project (`rustyfarian-rgb-clock`) highlighted an integration friction point:
their `clock-core` crate uses `type Rgb = (u8, u8, u8)` while `led-effects` uses `rgb::RGB8`, requiring conversion code at the boundary.

The question is whether to:

1. Keep the `rgb` crate dependency (current state)
2. Define our own `Rgb8` type to remove the external dependency
3. Support both via a conversion trait

## Decision

**Option A: Keep `rgb::RGB8` (Recommended)**

Continue using the `rgb` crate as the standard color type across all crates.

**Option B: Define Custom `Rgb8` Type**

Create a simple struct in `ws2812-pure` or a new `rgb-types` crate:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
```

**Option C: Support Both via `From` Traits**

Keep `rgb::RGB8` as primary but implement conversions:

```rust
impl From<(u8, u8, u8)> for Rgb8 { ... }
impl From<Rgb8> for (u8, u8, u8) { ... }
```

## Analysis

| Factor                      | Option A (Keep `rgb`)            | Option B (Custom Type)                       | Option C (Both)             |
|:----------------------------|:---------------------------------|:---------------------------------------------|:----------------------------|
| **Ecosystem compatibility** | Excellent - `rgb` is widely used | Poor - requires conversion at every boundary | Good - explicit conversions |
| **Dependency count**        | +1 external dep                  | No external dep                              | +1 external dep             |
| **Maintenance burden**      | None - maintained upstream       | Low - trivial type                           | Low - conversion impls      |
| **API surface**             | Rich (`RGB8` has many methods)   | Minimal (only what we define)                | Same as A                   |
| **Compile time impact**     | Negligible (~0.1s)               | None                                         | Negligible                  |
| **Breaking change**         | None                             | Yes - all downstream code                    | None                        |
| **`no_std` support**        | Yes (`rgb` is `no_std`)          | Yes                                          | Yes                         |

### Feedback from rustyfarian-rgb-clock

The downstream project noted:

> "The `clock-core` crate uses `type Rgb = (u8, u8, u8)` while `led-effects` uses `rgb::RGB8`.
> This requires conversion when mixing the two."

Their suggestion was to standardize on `rgb::RGB8` across all crates, which supports Option A.

### The `rgb` Crate

- Stable, widely used (2M+ downloads)
- `no_std` compatible
- Provides useful traits: `Default`, `From<[u8; 3]>`, `Into<[u8; 3]>`, `ComponentSlice`
- Version 0.8.x has been stable since 2021

## Consequences

### If Option A (Keep `rgb::RGB8`)

**Positive:**

- No breaking changes for existing users
- Ecosystem compatibility with other LED/graphics libraries
- Rich API surface for color manipulation
- Downstream projects can standardize on the same type

**Negative:**

- External dependency (though minimal and stable)
- Downstream projects using tuple types need conversion

**Migration path for downstream:**

Downstream projects like `clock-core` should adopt `rgb::RGB8` instead of custom tuple types.

### If Option B (Custom Type)

**Positive:**

- Zero external dependencies
- Full control over the type definition

**Negative:**

- Breaking change for all current users
- Loss of ecosystem compatibility
- Must implement all desired traits manually
- Downstream projects still need conversion code

### If Option C (Both)

**Positive:**

- Flexibility for downstream projects
- No breaking changes
- Easy migration path

**Negative:**

- Increased API surface
- May encourage inconsistent usage patterns

## Recommendation

**Option A: Keep `rgb::RGB8`**

The `rgb` crate is stable, `no_std` compatible, and widely adopted.
The downstream feedback explicitly suggested standardizing on `rgb::RGB8`.
Removing it would be a breaking change with minimal benefit.

Downstream projects should adopt `rgb::RGB8` rather than custom tuple types.
This aligns with the Rust ecosystem convention for color handling in embedded projects.
