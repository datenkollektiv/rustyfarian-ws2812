# WS2812 Embedded Rust Ecosystem Comparison

Analysis of comparable open-source crates for WS2812/addressable LED control,
evaluated against the `rustyfarian-ws2812` design philosophy.

## Executive Summary

- The `smart-leds-rs` ecosystem provides the de-facto Rust standard for LED driver interoperability via `smart-leds-trait`, but covers only the hardware output layer — no animation or effect logic, no host-runnable unit tests.
- `esp-hal-smartled` (both the `esp-rs/esp-hal-community` version and the `kleinesfilmroellchen` fork) are thin RMT glue layers — they do not separate pure logic into testable sub-crates and contain no unit tests at all.
- `ws2812-esp32-rmt-driver` is the only widely used competitor that explicitly supports host-testability, using a mock module and `--target x86_64-unknown-linux-gnu --lib` — but the mock is a low-level stub, not an architecturally distinct pure-logic layer.
- No examined crate provides ring-specific animations (rainbow, pulse, spinner, progress) in a standalone testable crate.
- `rustyfarian-ws2812` is unique in the ecosystem for making unit-testable pure logic the **primary design constraint** rather than an afterthought, with 139+ passing tests runnable on any laptop without an ESP32 or ESP toolchain.

---

## Per-Crate Analysis

<details>
<summary><strong>esp-hal-smartled (esp-rs/esp-hal-community)</strong></summary>

**Purpose and scope**

Community-maintained adapter crate in the `esp-rs/esp-hal-community` monorepo.
Provides a `SmartLedsAdapter` struct that wraps an ESP-HAL RMT channel and GPIO pin,
implementing the `SmartLedsWrite` trait from `smart-leds-trait`.
Targets the `no_std` bare-metal `esp-hal` stack (not ESP-IDF).
Supports blocking and async modes, SK6812/WS2812 timing constants, and RGB/RGBW color models.

**Architecture: pure logic separation**

None.
The approximately 403-line `src/lib.rs` contains hardware constants, hardware adapters,
and color-to-pulse conversion functions all in one file.
The conversion functions are hardware-agnostic in principle but are not extracted to a separate crate.
There is no workspace structure separating logic from hardware.

**Test coverage**

No tests present.
The project relies on examples and integration testing by consuming applications.
There is no mechanism to run any tests without an ESP32 target.

**API design observations**

The `SmartLedsAdapter` wraps an RMT channel and GPIO pin.
The `smart_led_buffer!` macro allocates a fixed-size buffer at compile time.
The API correctly delegates color format conversion to the `smart-leds` ecosystem,
keeping the hardware adapter thin in intent, though not in practice.

**Maintenance status**

- Repository: `esp-rs/esp-hal-community`
- Stars: 44 | Forks: 32
- Latest version: 0.17.0 (November 2025)
- Download rate: approximately 950/month

</details>

<details>
<summary><strong>esp-hal-smartled (kleinesfilmroellchen fork, published as esp-hal-smartled2)</strong></summary>

**Purpose and scope**

An enhanced fork, renaming the main struct to `RmtSmartLeds`.
Key additions: async support via `SmartLedsWriteAsync`, no-allocation design using only static buffers,
generic color type parameters (beyond RGB8 to support RGBW, 16-bit variants),
and compile-time type-level LED specifications.
Supports WS2811 (low/high speed), WS2812, WS2812B timing.

**Architecture: pure logic separation**

None.
Like the community crate, this is a single-crate driver.
Strong compile-time configuration via type parameters is a quality improvement
but does not constitute architectural separation of concerns in the testability sense.

**Test coverage**

No unit tests are visible.
Examples serve as the only demonstrable correctness evidence.
Tests cannot be run without ESP32 hardware and toolchain.

**API design observations**

The most sophisticated API of the RMT-based adapters is examined.
Type parameters encode LED count (`BUFFER_SIZE`), color type, color order, and timing at compile time,
enabling zero-cost abstractions and compile-time correctness.
The `buffer_size::<RGB8>(N)` helper calculates the correct buffer size.
Convenience type aliases (`Rgb8RmtSmartLeds`, `Ws2812Timing`) reduce verbosity.

**Maintenance status**

- Repository: `kleinesfilmroellchen/esp-hal-smartled`
- Stars: 5 | Forks: 4
- Latest version: 0.28
- Small user base but appears carefully maintained

</details>

<details>
<summary><strong>smart-leds-trait + smart-leds (smart-leds-rs organization)</strong></summary>

**Purpose and scope**

`smart-leds-trait` defines the `SmartLedsWrite` and `SmartLedsWriteAsync` traits.
Intentionally minimal — a single `no_std` file of approximately 70 lines defining the two traits plus pixel types.
`smart-leds` is the end-user crate providing utility iterators (`brightness()`, `gamma()`) and color types,
re-exporting the trait.
Neither crate implements any animation or effect logic.

**Architecture: pure logic separation**

`smart-leds-trait` itself is an excellent example of a pure-logic artifact: no hardware dependencies,
compiles on any target.
However, it is a trait definition only — no logic to separate.
The organization stops at the driver output layer.

**Test coverage**

No explicit tests in either crate.
Testing is expected to happen in driver implementations and consumer code.

**API design observations**

The trait design is mature and well-thought-out.
The use of associated types for `Color` and `Error` allows each driver to express its own color model.
The `IntoIterator<Item = I>` + `I: Into<Self::Color>` double-generic pattern allows color type flexibility.
The `gamma()` and `brightness()` iterators in `smart-leds` compose cleanly with any driver.
This is the strongest ecosystem foundation examined.

**Maintenance status**

- `smart-leds-trait`: latest v0.3.2 (September 2025), 6 stars
- `smart-leds`: latest v0.4.0, 134 stars
- Organization `smart-leds-rs`: 10 repositories, actively maintained

</details>

<details>
<summary><strong>ws2812-spi (smart-leds-rs/ws2812-spi-rs)</strong></summary>

**Purpose and scope**

`embedded-hal` driver for WS2812 LEDs using SPI as the timing mechanism.
Provides three variants: normal (real-time on-the-fly SPI data generation),
prerendered (pre-computed buffers for slower processors),
and hosted (single-call transmission for Linux SBCs).
Implements `SmartLedsWrite`.

**Architecture: pure logic separation**

Moderate.
The `embedded-hal` dependency provides a hardware abstraction boundary.
However, the crate does not provide a dedicated pure-logic sub-crate,
does not separate color encoding math from SPI formatting,
and does not ship mock implementations.

**Test coverage**

Minimal.
No dedicated test directory is visible.
No host-runnable test suite is documented.

**Maintenance status**

- Stars: 98 | Forks: 31
- Latest: v0.5.1 (June 2025)
- 83 commits, active and stable

</details>

<details>
<summary><strong>ws2812-esp32-rmt-driver (cat-in-136)</strong></summary>

**Purpose and scope**

WS2812B driver using the ESP32 RMT peripheral, targeting the ESP-IDF stack.
Provides three API layers via feature flags: a direct low-level driver, a `smart-leds-trait` wrapper,
and an `embedded-graphics-core` drawing target.
Supports SK6812-RGBW.

**Architecture: pure logic separation**

Partial — and uniquely documented.
Ships "mock modules for local testing" that simulate the low-level ESP-IDF RMT API on non-ESP platforms.
This is the only crate in the comparison that ships a formal mock layer.
However, the mock is a low-fidelity stub of the hardware API, not an architecturally distinct pure-logic layer.
The result is testability-by-substitution rather than testability-by-separation.

**Test coverage**

Host-testable via:

```sh
cargo +stable test --target x86_64-unknown-linux-gnu --lib
```

Explicitly documented and appears to be a first-class feature.
This is a meaningful advantage over all other hardware-specific crates examined.

**API design observations**

The most feature-rich driver examined.
The `embedded-graphics-core` integration is a unique addition enabling 2D drawing abstractions over LED matrices.

**Maintenance status**

- Stars: 64 | Forks: 32
- Latest: v0.13.1 (October 2025)
- 190 commits — the most actively developed ESP32-specific driver

</details>

<details>
<summary><strong>smart_led_effects (bitbrain-za)</strong></summary>

**Purpose and scope**

Collection of 15 animation effects for individually addressable LED strips/rings:
Breathe, Bounce, Collision, Cylon, Fire, Meteor, Morse, ProgressBar, Rainbow,
RunningLights, SnowSparkle, Strobe, Timer, Twinkle, Wipe.
Returns `Vec<Srgb>` per frame.

**Architecture: pure logic separation**

Good within its scope.
Effects return color vectors that consumers send to their driver of choice — no hardware dependencies at all.
The `EffectIterator` trait defines a minimal interface.
Architecturally aligned with the `rustyfarian-ws2812` philosophy.
However, use of `Vec<Srgb>` (heap allocation) makes it `std`-only — unsuitable for `no_std`/no-alloc targets.

**Test coverage**

Not documented.
No test infrastructure is visible.

**Maintenance status**

- Stars: 7 | Forks: 5
- Latest: v0.1.7 (December 2024)
- Pre-release stability (0.x versioning)

</details>

<details>
<summary><strong>smart_leds_animations</strong></summary>

**Purpose and scope**

Declarative animation framework for the `smart-leds` ecosystem.
Provides a `Director`/`Driver`/`AnimateFrames` theatrical abstraction.
Built-in animations: `Snake`, `Arrow`.
Composition primitives: `Parallel` and `Series`.
Zero heap allocations, `no_std` compatible.

**Architecture: pure logic separation**

Good in intent.
Animations reference pixel indices, not hardware; the `Driver` handles gamma/brightness and hardware communication.
However, the animation logic and orchestration framework are tightly coupled —
there is no separate pure-math crate and no independent animation state machine.

**Test coverage**

Not documented.
Author acknowledges limited real-world testing (WS2812B strip on Arduino Uno R3 only).

**Maintenance status**

- Latest: v0.1.0 (May 2025)
- Early-stage, single release

</details>

---

## Comparison Table

| Crate                                     | Pure/Testable Logic Layer                                           | HW Abstraction Quality                             | Animation Support                                | Tests on Host                                 | Maintenance                      |
|:------------------------------------------|:--------------------------------------------------------------------|:---------------------------------------------------|:-------------------------------------------------|:----------------------------------------------|:---------------------------------|
| **rustyfarian-ws2812** (this project)     | Yes — `ws2812-pure`, `ferriswheel`, `led-effects` with zero HW deps | Thin RMT glue in separate crates                   | Yes — rainbow, pulse, spinner, progress, section | Yes — 139+ tests, `just test`                 | Active                           |
| `esp-hal-smartled` (esp-rs community)     | No — single file, logic interleaved with HW                         | Wraps esp-hal RMT, delegates to `smart-leds-trait` | None                                             | No — requires ESP32 target                    | Active (0.17.0, Nov 2025)        |
| `esp-hal-smartled` (kleinesfilmroellchen) | No — single crate, no workspace                                     | Better: generic type params for color/timing       | None                                             | No — requires ESP32 target                    | Active, small user base          |
| `smart-leds-trait` + `smart-leds`         | Trait-only (no logic to separate)                                   | Excellent trait design, ecosystem foundation       | None                                             | N/A — no logic                                | Active (Sep 2025)                |
| `ws2812-spi`                              | No dedicated layer — relies on embedded-hal abstraction             | Good — programs against `embedded-hal` traits      | None                                             | Partial via mock HAL                          | Active (Jun 2025)                |
| `ws2812-esp32-rmt-driver`                 | Partial — ships mock modules for host compilation                   | Moderate — mock enables host tests                 | None                                             | Yes — documented `--lib --target x86` pattern | Most active (Oct 2025, 64 stars) |
| `smart_led_effects`                       | Yes — fully HW-agnostic, returns `Vec<Srgb>`                        | N/A — driver-agnostic                              | Yes — 15 effects                                 | Not documented                                | Moderate (Dec 2024)              |
| `smart_leds_animations`                   | Partial — animation logic HW-agnostic                               | Clean callback injection for sleep                 | Yes — Snake, Arrow, Parallel/Series              | Not documented                                | Early (May 2025)                 |

---

## Strategic Recommendation

### Adopt from the ecosystem

**Implement `SmartLedsWrite` in the hardware wrapper crates.**
The `smart-leds-trait` design is mature and widely adopted.
Implementing it in `rustyfarian-esp-idf-ws2812` and `rustyfarian-esp-hal-ws2812`
would allow consumers to use the `brightness()` and `gamma()` iterators from `smart-leds` without conversion code,
and enable future compatibility with `smart-leds`-based effect crates.

**Note `ws2812-esp32-rmt-driver`'s mock pattern as validation.**
That crate independently arrived at the same conclusion: hardware-level code must be substitutable for host tests to be meaningful.
This project's approach — trait abstraction in `led-effects` plus separate pure-logic crates —
is the more principled version of the same insight (testability-by-separation vs. testability-by-substitution).

**Consider `kleinesfilmroellchen`'s compile-time buffer sizing.**
The `buffer_size::<Color>(N)` pattern catches buffer sizing errors at compile time
and is worth adopting in the ESP-HAL wrapper.

### What this project does that no examined crate does

**Ring-specific geometry in a standalone testable crate** is unique to `ferriswheel`.
Every other animation crate treats LEDs as a linear strip.
Ring topology (wrap-around indexing, angular position, radial symmetry) requires different primitives,
and isolating those in a `no_std`, host-testable crate with 111 passing unit tests is not replicated anywhere in the ecosystem.

**Three-layer architecture as a first-class design constraint.**
The `ws2812-pure` / `ferriswheel` / `led-effects` / ESP-driver layering
is the only examined approach where testability on a laptop is enforced from the beginning, not retrofitted.

### Effect breadth backlog

`smart_led_effects` covers 15 effects (Breathe, Cylon, Fire, Meteor, Strobe, Twinkle, etc.)
versus `ferriswheel`'s 4 highly tested effects.
The current approach prioritizes correctness and test coverage over breadth — the right choice for a foundation library.
The `smart_led_effects` source list is a useful backlog if effect breadth becomes a future goal.

### Maintain our own approach

The core `rustyfarian-ws2812` architecture is not replicated by any examined crate
and represents a genuine gap in the ecosystem.
Contributing the pure-logic layer upstream (e.g., proposing animation sub-crates to `smart-leds-rs`)
is a valid long-term option once the APIs stabilize,
but maintaining control over the pure-logic layer is the correct near-term strategy.

---

*Research conducted February 2026.*
*Sources: GitHub repositories, docs.rs, crates.io, lib.rs.*
