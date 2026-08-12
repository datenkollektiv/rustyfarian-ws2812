# Blinksy Ecosystem Evaluation

*Research date: 2026-05-05*

---

## Summary

- **`blinksy` is a spatially-oriented LED framework**, not an animation-effect library.
  Its core abstraction is a normalised coordinate system for 1D/2D/3D LED layouts; effects compute colours per pixel using position and elapsed time.
  This is architecturally different from `ferriswheel`, which provides a fixed vocabulary of ring effects behind a stateful `Effect` trait.
- **The EUPL-1.2 licence is the single biggest practical barrier** to adopting `blinksy` as a dependency.
  It is a copyleft licence that is not on the list of licences compatible with MIT or Apache-2.0.
  Including it as a Cargo dependency in a project that ships under MIT/Apache-2.0 carries legal risk that legal counsel would need to evaluate.
- **There is no crates.io name collision** for `bunting`, `pennant`, or `ferriswheel` against the `blinksy` namespace.
  `blinksy` does not re-export or shadow any of our planned crate names.
- **The projects fill genuinely different niches**: `blinksy` is framework-complete for spatial LED installations (art, lighting controllers); `ferriswheel` is focused on small embedded rings with a simple, testable effect loop.
  They are complementary rather than redundant.
- **`blinksy` as an upstream contribution target is a poor fit**: the licence incompatibility and the different design philosophy (spatial-generic vs ring-specific, no separation of pure logic from hardware) make it a weaker choice than the existing `smart-leds-rs` roadmap item.

---

## What `blinksy` Is

`blinksy` (v0.11.0, May 2026) is a `no_std`, `no_alloc` Rust LED control library licensed under EUPL-1.2.
It describes itself as inspired by FastLED and WLED, targeting 1D, 2D, and 3D LED spatial installations.

### Feature Surface

The library is structured as four published crates plus a board-support package:

| Crate | Role | Version |
|:------|:-----|:--------|
| `blinksy` | Core: layout, pattern, driver, colour, control | 0.11.0 |
| `blinksy-esp` | ESP32 HAL integration (esp-hal 1.0.0-rc.1) | 0.11.0 |
| `blinksy-desktop` | Desktop simulation using egui + miniquad | 0.11.0 |
| `gledopto` | Board-support for Gledopto GL-C-016WL-D | — |

### Key Traits

**`Pattern<Dim, Layout>`** — the central abstraction:

```rust
pub trait Pattern<Dim, Layout>
where
    Layout: LayoutForDim<Dim>,
{
    type Params;
    type Color;
    fn new(params: Self::Params) -> Self;
    fn tick(&self, time_in_ms: u64) -> impl Iterator<Item = Self::Color>;
}
```

`tick()` takes a timestamp and returns one colour per LED as an iterator.
There is no mutation of internal state on `tick()` — the pattern is stateless and recomputes from the timestamp.

**`Layout1d` / `Layout2d`** — spatial mapping traits:

```rust
pub trait Layout1d {
    const PIXEL_COUNT: usize;
    fn points() -> impl Iterator<Item = f32>;
}

pub trait Layout2d {
    const PIXEL_COUNT: usize;
    fn shapes() -> impl Iterator<Item = Shape2d>;
    fn points() -> impl Iterator<Item = Vec2>;
}
```

`Layout1d` maps LEDs to positions in `[-1.0, 1.0]`.
`Layout2d` uses `glam::Vec2` coordinates in `[-1.0, 1.0] × [-1.0, 1.0]`.
Both are generated via macros (`layout1d!`, `layout2d!`).
`Layout2d` supports `Shape2d::Grid` (with `serpentine` flag for zigzag wiring) and `Shape2d::Arc` (which can represent a full circle by spanning TAU).
There is no dedicated ring or circle layout type — a ring must be approximated as a full-arc shape.
`Layout3d` is described as "coming soon".

**`Driver`** — hardware abstraction:

```rust
trait Driver {
    type Error;
    type Color;
    type Word;
    fn encode(...) -> Vec<Self::Word, FRAME_BUFFER_SIZE>;
    fn write(...) -> Result<(), Self::Error>;
    fn show(...) -> Result<(), Self::Error>; // default: encode then write
}
```

Driver implementations cover clockless LEDs (WS2812B, SK6812) and clocked LEDs (APA102).
The `Driver` trait does not reference `embedded-hal` directly.

**`ControlBuilder`** — fluent assembly API:

```rust
ControlBuilder::new_1d()         // or new_2d(), new_3d()
    .with_layout::<MyLayout, PIXEL_COUNT>()
    .with_pattern::<Rainbow>(params)
    .with_driver(driver)
    .with_frame_buffer_size::<FRAME_BUFFER_SIZE>()
    .build()
```

The builder uses phantom type parameters and `Set`/`Unset` marker traits to enforce compile-time completeness.
The resulting `Control` struct exposes `tick(time_in_ms: u64)` (blocking) or `.await`-able `tick()` (async).

### Built-in Patterns

Only two built-in patterns exist at v0.11.0: **Rainbow** and **Noise**.
Both implement `Pattern` for 1D, 2D, and (once 3D ships) 3D.
Rainbow uses `Hsv<HsvHueRainbow>` and computes hue from spatial coordinate plus elapsed time.
Noise uses procedural noise from the `noise-functions` crate.

### Colour System

The colour system is inspired by the `palette` crate, supporting `Hsv` (FastLED rainbow hue space), `Okhsv` (perceptually uniform), and `Rgb8`.
Global brightness (f32) and colour correction (`ColorCorrection`) are applied in `Control::tick()` before writing to the driver.

### Target Hardware and Async

`blinksy-esp` targets ESP32 variants via `esp-hal 1.0.0-rc.1` with chip feature flags (`esp32c2`, `esp32c3`, `esp32c6`, `esp32h2`, `esp32`, `esp32s2`, `esp32s3`).
Async is an optional feature using `embedded-hal-async`.
The project lists RP2040, STM32, nRF, atsamd, AVR, and CH32 as planned targets (open issues exist for each).

### Desktop Simulation

`blinksy-desktop` renders LED layouts and patterns in a 3D graphical window using `egui` (0.28) and `miniquad` (0.4).
This is a qualitatively different approach to host-testability than unit tests: it is visual simulation rather than assertion-based testing.

### Test Coverage

No `#[cfg(test)]` unit tests are visible in the repository structure.
The workspace does not contain a `tests/` directory.
Host-testability in `blinksy` is addressed exclusively through the desktop simulation path, not through assertable test functions.

### Maintenance Status

v0.11.0, released 2025–2026 (active development).
The repository shows 34 open issues and 4 open pull requests.
The author (`ahdinosaur`) is actively engaged.
AVR driver support is an open issue (help wanted); it does not yet exist in the crate.

---

## API Comparison: `blinksy` vs `ferriswheel`

<details>
<summary><strong>Side-by-side trait and pattern comparison</strong></summary>

| Dimension | `blinksy` (v0.11.0) | `ferriswheel` |
|:----------|:--------------------|:--------------|
| Core abstraction | `Pattern<Dim, Layout>`: spatial, position-driven | `Effect` trait: ring-aware, stateful effect loop |
| State model | Stateless: `tick(&self, ms) -> Iterator<Color>` — recomputes from timestamp | Stateful: `update(&mut self, buf: &mut [RGB8])` advances internal animation state |
| Animation timing | Caller passes `time_in_ms: u64`; pattern is pure function of time | Internal step counter advanced on each `update()` call; speed controlled by `with_speed()` |
| Buffer ownership | Pattern produces an iterator; `Control` owns the frame buffer (`heapless::Vec`) | Caller owns the output buffer (`&mut [RGB8]`); effect writes into it |
| Construction | `ControlBuilder` fluent API with phantom-type state machine | Builder methods (`with_speed`, `with_brightness`, `with_direction`); `new()` returns `Result<Self, EffectError>` |
| Geometry | Any 1D (linear), 2D (grid, arc, etc.), 3D (planned) spatial layout | Ring-specific: LED count as primary parameter; all effects assume circular topology |
| Built-in patterns | 2 (Rainbow, Noise) | 14 (Rainbow, Pulse, Breathe, Spinner, Meteor, Twinkle, Fire, Cylon, KnightRider, Chase, Flash, Progress, Section, RainbowComet) |
| Ring-native effects | No ring concept; a ring must be defined as a full-arc `Shape2d` | All 14 effects are designed for ring topology (wrap-around, direction, positional indexing) |
| Test approach | Visual desktop simulation (no unit tests visible) | 316 `#[cfg(test)]` unit tests + 19 doc tests, host-runnable without hardware |
| `no_std` | Yes | Yes |
| `no_alloc` | Yes (frame buffer is `heapless::Vec`) | Yes (zero allocation; caller owns output buffer) |
| Host testing | Desktop simulation via `blinksy-desktop` | `cargo test --target <host>` runs full test suite |
| Async support | Optional feature (`embedded-hal-async`) | Optional feature (Embassy-based; `rustyfarian-esp-hal-ws2812` only) |
| `smart-leds-trait` | Listed as a dependency (`smart-leds-trait 0.3.1`) but trait usage not prominent | `SmartLedsWrite` implemented on blocking HAL driver; `SmartLedsWriteAsync` roadmap item |
| Licence | EUPL-1.2 (weak copyleft; incompatible with MIT/Apache-2.0 as direct dependency) | MIT / Apache-2.0 |
| Colour spaces | `Hsv`, `Okhsv`, `Rgb8`; FastLED rainbow hue | `rgb::RGB8`; HSV computed via internal tables |
| Dependencies | `glam`, `heapless`, `fugit`, `noise-functions`, `num-traits` | `rgb` only (no_std pure crates); HAL crates are in driver crates |

</details>

### Where the approaches align

Both crates are `no_std` and `no_alloc`.
Both expose a trait-based abstraction for defining visual effects.
Both allow callers to drive the animation loop externally.
Both target embedded hardware as the primary deployment environment.

### Where the approaches diverge

The fundamental difference is **spatial generality vs ring specificity**.

`blinksy` treats every LED as a point in normalised space.
A pattern computes colour as a function of position and time, making it equally applicable to a strip, a grid, or a sphere of LEDs.
This is the right model for installations where the geometry of the LED arrangement is the primary variable.

`ferriswheel` treats the LED array as a ring.
Effects know they are working on a circular buffer: `SpinnerEffect` wraps around, `CylonEffect` bounces, `MeteorEffect` trails across a cylinder.
This specificity is what makes it possible to write 14 semantically distinct, visually appropriate ring effects rather than two geometry-agnostic ones.

The state model also differs materially.
`blinksy`'s `tick(&self, ms)` means the pattern has no history — the same timestamp always produces the same output.
This makes patterns deterministic and composable but means temporal effects (decay, momentum, fade-out) require the caller to manage elapsed time carefully.
`ferriswheel`'s stateful `update()` means the effect carries its own momentum: a meteor's tail decays naturally across frames without the caller tracking it.

For a caller who wants exactly 14 well-tested ring effects with minimal setup, `ferriswheel` is simpler.
For a caller who wants to define a custom spatial layout (asymmetric strip, multi-segment installation, matrix), `blinksy` is more flexible but requires more configuration.

---

## Overlap with Our `grid` Module

The `README.md` and `ROADMAP.md` mention a `GridBuffer` and `GridLayout` as a 2D matrix abstraction added in commit `5804d99`.
`blinksy`'s `Layout2d` with `Shape2d::Grid` covers this same ground.

### What `blinksy` does better

`blinksy`'s grid model is more compositional: multiple shapes (grids, arcs, free-form point sets) can be combined into a single layout.
The serpentine flag handles zigzag wiring natively.
Coordinate normalisation to `[-1.0, 1.0]` makes effects geometry-independent.

### What our grid module does (or should do)

Our `GridBuffer` and `GridLayout` are presumably closer to a direct row/column addressing model.
Without inspecting the commit directly, the likely advantage is simpler indexing (`(row, col) → linear index`) and the ability to use existing `ferriswheel` effects adapted for matrix form.

### Recommendation on the grid module

The EUPL-1.2 licence means `blinksy`'s grid abstractions cannot be adopted as a Cargo dependency without licence review.
Continue evolving our own grid abstractions independently.
The `blinksy` design is worth studying as a reference — particularly the serpentine flag and the normalised coordinate model — but the code cannot be copied (EUPL-1.2 source disclosure obligations apply).
If the grid module gains enough scope to warrant its own crate, evaluate then whether a licence-compatible grid-layout library exists (e.g. from the `embedded-graphics` ecosystem).

---

## Implications for Our Project

### README pitch

The current README frames the project as filling an ecosystem gap with testable pure logic.
`blinksy`'s existence narrows the gap slightly: it is also `no_std` and `no_alloc`.
However, the gap it does not fill is:

1. **Assertion-based unit tests runnable on a laptop** — `blinksy` has none visible; `ferriswheel` has 316.
2. **Ring-specific effect vocabulary** — `blinksy` has 2 geometry-agnostic patterns; `ferriswheel` has 14 ring-native effects.
3. **MIT/Apache-2.0 licence** — `blinksy` is EUPL-1.2; our crates are dual-licensed permissively.
4. **AVR support** — `blinksy` lists AVR as a help-wanted open issue; we ship a working AVR driver.

The pitch should be updated to acknowledge that `blinksy` exists and to sharpen the positioning.
The existing sentence "Most embedded LED libraries require a device to verify even pure logic" remains accurate for `blinksy`.
Consider adding a sentence along the lines of:

> `ferriswheel` differs from spatially-oriented frameworks such as `blinksy` in three ways:
> it assumes ring topology, it carries 300+ unit tests runnable on a laptop, and it is permissively licensed (MIT/Apache-2.0).

Do not dismiss `blinksy` — it is a legitimate, well-designed library for a different use case.
Positioning as complementary is more accurate and more credible than positioning as superior.

### Roadmap: upstream contribution evaluation

The `ROADMAP.md` long-term item reads: "Evaluate upstream contribution to `smart-leds-rs`".
`blinksy` should **not** replace `smart-leds-rs` as the upstream contribution target.

Reasons:

1. The EUPL-1.2 licence requires any modifications to `blinksy` to be released under EUPL-1.2.
   Contributing our ring effects to `blinksy` would effectively relicence that contribution away from MIT/Apache-2.0.
2. `blinksy`'s design philosophy (position-driven, stateless patterns) is architecturally incompatible with `ferriswheel`'s stateful ring effects.
   A direct port would require fundamental redesign, not contribution.
3. `smart-leds-rs` is MIT/Apache-2.0 compatible and specifically targets the embedded LED hardware interface layer that our driver crates already implement (`SmartLedsWrite`).

The `smart-leds-rs` upstream contribution path remains the better long-term target.
`blinksy` is worth monitoring as an independent parallel ecosystem, but not a contribution destination.

### Name collision check

- `bunting` — not present on crates.io; not used by `blinksy` or its sub-crates. **No collision.**
- `pennant` — verified free on crates.io (HTTP 200 check, 2026-05-06); not used by `blinksy`. **No collision.** Selected after `lantern` was found taken — the earlier "free" claim was based on a `cargo search latern` typo.
- `ferriswheel` — no crates.io results; not used by `blinksy`. **No collision.** Re-verify with `cargo search ferriswheel` before first publish as the feature doc requires.

---

## Recommendations

Ranked by urgency.

**1. Update `why-yet-another-ws2812-crate.md` to acknowledge `blinksy`.**
Add a paragraph noting that `blinksy` occupies adjacent territory with a spatial-layout model, and sharpen the differentiation: assertion-based tests, ring-specific effects, and permissive licensing are the three distinctives that `blinksy` does not cover.
This keeps the framing honest and makes the project's niche more precise.
*Action:* Brief prose edit to `docs/why-yet-another-ws2812-crate.md` (do not rename or restructure — just add the paragraph).

**2. Add a sentence to the README that positions `ferriswheel` relative to `blinksy`.**
The README currently makes no mention of `blinksy`.
Acknowledge it as a complementary library for spatial installations, and state that `ferriswheel` focuses on ring-topology effects with host-runnable tests and permissive licensing.
*Action:* One or two sentences in the "Rustyfarian Philosophy" section or a new "Ecosystem Context" callout.

**3. Do not adopt `blinksy` as a Cargo dependency.**
The EUPL-1.2 licence is incompatible with the project's MIT/Apache-2.0 dual licence.
The architectural mismatch (spatial vs ring, stateless vs stateful) means there is little to gain beyond the licence risk.
*Action:* Record this decision in this document (done) and in `docs/project-lore.md` under a new "Dependency Licence Decisions" entry if similar questions arise in future.

**4. Keep the upstream contribution roadmap item pointed at `smart-leds-rs`, not `blinksy`.**
No change to `ROADMAP.md` is needed — the existing wording is correct.
*Action:* None required.

**5. Mark the `blinksy` open question in the crates.io publication feature doc as resolved.**
The feature doc (`docs/features/archive/crates-io-publication-v1.md`) listed the `blinksy` evaluation as a queued follow-up.
This document closes that question.
The conclusion is: `blinksy` does not change the publish scope, the crate names, or the positioning in any way that blocks v1 publication.
*Action:* In the feature doc, check the open question item and add a note pointing to this document.

---

## Sources

- [blinksy — crates.io](https://crates.io/crates/blinksy)
- [blinksy-esp — crates.io](https://crates.io/crates/blinksy-esp)
- [blinksy-desktop — crates.io](https://crates.io/crates/blinksy-desktop)
- [ahdinosaur/blinksy — GitHub](https://github.com/ahdinosaur/blinksy)
- [blinksy — docs.rs](https://docs.rs/blinksy/latest/blinksy/)
- [blinksy-desktop — docs.rs](https://docs.rs/blinksy-desktop/latest/blinksy_desktop/)
- [First look at Blinksy — blog.mikey.nz](https://blog.mikey.nz/first-look-at-blinksy/)
- [Blinksy: Rust no-std, no-alloc LED control library — Hacker News](https://news.ycombinator.com/item?id=44109595)
- [European Union Public License 1.2 — choosealicense.com](https://choosealicense.com/licenses/eupl-1.2/)
- [But ultimately, how copyleft is the EUPL? — Interoperable Europe Portal](https://interoperable-europe.ec.europa.eu/collection/eupl/discussion/ultimately-how-copyleft-eupl)
- [blinksy/src/pattern.rs — GitHub](https://github.com/ahdinosaur/blinksy/blob/main/blinksy/src/pattern.rs)
- [blinksy/src/control.rs — GitHub](https://github.com/ahdinosaur/blinksy/blob/main/blinksy/src/control.rs)
- [blinksy/src/layout/layout1d.rs — GitHub](https://github.com/ahdinosaur/blinksy/blob/main/blinksy/src/layout/layout1d.rs)
- [blinksy/src/layout/layout2d.rs — GitHub](https://github.com/ahdinosaur/blinksy/blob/main/blinksy/src/layout/layout2d.rs)
- [blinksy/src/patterns/rainbow.rs — GitHub](https://github.com/ahdinosaur/blinksy/blob/main/blinksy/src/patterns/rainbow.rs)
- [blinksy/src/lib.rs — GitHub](https://github.com/ahdinosaur/blinksy/blob/main/blinksy/src/lib.rs)
- [blinksy/Cargo.toml — GitHub](https://github.com/ahdinosaur/blinksy/blob/main/blinksy/Cargo.toml)
- [blinksy-desktop/Cargo.toml — GitHub](https://github.com/ahdinosaur/blinksy/blob/main/blinksy-desktop/Cargo.toml)
- [esp/blinksy-esp/Cargo.toml — GitHub](https://github.com/ahdinosaur/blinksy/blob/main/esp/blinksy-esp/Cargo.toml)
- [ahdinosaur/blinksy — open issues](https://github.com/ahdinosaur/blinksy/issues)
