# Roadmap

*Last updated: March 2026*

This roadmap is informed by the [ecosystem comparison](ecosystem-comparison.md) conducted in February 2026.
Items are grouped by theme and ordered roughly by impact and dependency.
Vision review (March 2026) refocused near-term priority on `NoLed` → `esp-hal` driver → ecosystem integration.
`NoLed`, the `esp-hal` driver, `SmartLedsWrite`, and `BreatheEffect` are now complete; near-term focus has shifted to adopting `smart-leds-trait` color types in the pure crates.

```mermaid
%%{init: {
  "theme": "base",
  "themeVariables": {
    "cScale0": "#c8f7c5",
    "cScaleLabel0": "#1b5e20",
    "cScale1": "#fff3cd",
    "cScaleLabel1": "#7a5a00",
    "cScale2": "#e3f2fd",
    "cScaleLabel2": "#0d47a1"
  }
}}%%

timeline
    title Fuzzy Rustyfarian WS2812 Roadmap

    Near term : SmartLedsWrite in hw wrappers (done)
              : Breathe effect (done)

    Mid term  : Adopt smart-leds color types (done)
              : Meteor / comet effect (after Breathe)
              : Twinkle / sparkle effect (after Meteor)
              : Fire effect (after Twinkle)
              : Cylon / bouncing scanner (after Fire)

    Long term : Upstream contribution evaluation (after Adopt smart-leds color types)
              : embedded-graphics-core evaluation (after Upstream contribution evaluation)
```

## Ecosystem Integration

<details>
<summary><strong>Implement <code>SmartLedsWrite</code> in hardware wrapper crates ✓ done</strong></summary>

`SmartLedsWrite` is now implemented in both `rustyfarian-esp-hal-ws2812` and `rustyfarian-esp-idf-ws2812`,
enabling use of the `brightness()` and `gamma()` iterator adapters from the `smart-leds` crate without any conversion code.
See the [Unreleased] CHANGELOG entry for details.

</details>

### Adopt `smart-leds-trait` color types in pure crates ✓ done

`smart-leds-trait v0.3.2` and `ferriswheel` both depend on `rgb v0.8` — the same
crate at the same version — so `rgb::RGB8` is already the same type on both sides.
No conversion or adapter code is needed.
The `idf_c6_smart_leds` and `hal_c6_smart_leds` examples confirm this empirically:
effect buffer output feeds directly into `SmartLedsWrite::write()` with zero glue.

Two small follow-on items are tracked below.

### Re-export `RGB8` from `ferriswheel` ✓ done

`pub use rgb::RGB8` added to `ferriswheel::lib`; callers can now write
`use ferriswheel::RGB8` without a direct `rgb` dependency.

### Guard against `rgb` version divergence between `ferriswheel` and `smart-leds-trait`

If `smart-leds-trait` ever bumps to `rgb v0.9`, the two `RGB8` types would silently
diverge and users would see confusing type-mismatch errors at the `SmartLedsWrite`
call site rather than a clean version-conflict at `cargo update` time.
Mitigation: add `smart-leds-trait` as an optional dependency in `ferriswheel`
(e.g. `smart-leds-compat = ["dep:smart-leds-trait"]`).
Cargo would then surface the version conflict at resolve time rather than at the
user's build site.
Purely defensive — not urgent until `smart-leds-trait` signals an `rgb` bump.

---

## Hardware Driver Improvements

<details>
<summary><strong>Completed hardware driver items (v0.2.0 – v0.3.0)</strong></summary>

**Implement `rustyfarian-esp-hal-ws2812` driver ✓ done (v0.3.0)**

Full WS2812 RMT driver using `esp-hal 1.0.0`, targeting ESP32-C6 (RISC-V, `riscv32imac-unknown-none-elf`), bare-metal `no_std`.
All color logic delegates to `ws2812-pure`; const-generic `N` buffer sizing was absorbed from the compile-time sizing item.

**Compile-time buffer sizing in the ESP-HAL wrapper ✓ absorbed into driver (v0.3.0)**

`buffer_size(num_leds)` const helper and `const N: usize` generic parameter ship as part of the driver.
Sizing errors are caught at compile time.

**Implement `NoLed` stub in `led-effects` ✓ done (v0.2.0)**

`NoLed` is a zero-size `StatusLed` implementor with `type Error = Infallible`,
satisfying the trait in applications that have no physical status LED.
See the v0.2.0 CHANGELOG entry for details.

</details>

---

## Animation Effects (`ferriswheel`)

The current `ferriswheel` crate provides eight well-tested, ring-specific effects:
`RainbowEffect`, `PulseEffect`, `SpinnerEffect`, `ChaseEffect`, `FlashEffect`, `ProgressEffect`, `SectionEffect`, and `BreatheEffect`.
The ecosystem survey identified the following as good candidates for the backlog —
each would need ring-geometry-aware implementation and full test coverage.

### Breathe effect ✓ done

A smooth, symmetric sinusoidal brightness envelope applied to a solid color.
`BreatheEffect` uses a full-wave sine: brightness rises continuously to the peak and descends symmetrically back to the minimum with no pause at the floor.
`PulseEffect` uses a half-wave rectified sine, which produces a heartbeat character — brightness rises to the peak, falls to zero, then *pauses at zero* for roughly a quarter of the cycle before rising again.

### Deferred follow-ups from BreatheEffect review

Small improvements deferred during the BreatheEffect review.
Not blocking, but tracked here to avoid being lost.

- **`current_brightness` underflow when `min > max`** ✓ done — `BreatheEffect` and `PulseEffect`
  now clamp via `u8::min`/`u8::max` before the range arithmetic; inverted ranges are
  treated identically to the correct order.

- **`PartialEq` derive on effect structs** — `BreatheEffect` (and `PulseEffect`) do not derive
  `PartialEq`, making test assertions verbose.
  Low priority; consistent with current `PulseEffect` behaviour; fix both at the same time.

- **Oversized-buffer acceptance test** — all effects silently accept buffers larger than
  `num_leds` and write only the first `num_leds` entries.
  This is intentional but untested.
  Add a shared contract test (or per-effect test) that confirms oversized buffers are accepted
  and only the required LEDs are written.

- **`PulseEffect` doc says "breathing cycle"** — the `PulseEffect` rustdoc uses the phrase
  "breathing cycle" even though `BreatheEffect` now owns that term.
  Rename to "pulse cycle" to reduce confusion.

### Fire effect

A heat-map simulation with randomised flickering decay from a "hot" base upward through the ring.
Requires a per-LED temperature buffer and a decay function.

### Meteor / comet effect

A bright "head" with a fading tail that travels around the ring.
Requires wrap-around index arithmetic already established in the ring geometry model.

### Twinkle / sparkle effect

Random LEDs briefly illuminate at peak brightness then decay.
Good for ambient/idle states.

### Cylon / bouncing scanner effect

A single bright LED (or small cluster) sweeps back and forth across the ring, automatically reversing direction.
Note: `ChaseEffect` already covers unidirectional scanning; this adds the auto-bounce behaviour.

---

## Long-term / Strategic

### Evaluate upstream contribution to `smart-leds-rs`

The pure-logic crates (`ws2812-pure`, `ferriswheel`) represent a gap in the ecosystem:
no existing `smart-leds-rs` crate provides ring-geometry animations testable without hardware.
Once the APIs are stable, evaluate whether proposing these as upstream additions or companion crates makes sense.
Decision should follow a stability review and user feedback.

### Evaluate `embedded-graphics-core` integration for matrix displays

`ws2812-esp32-rmt-driver` demonstrated a clean `embedded-graphics-core` drawing target
for addressing LEDs as a 2D pixel grid.
If a matrix display use-case emerges, this pattern provides a ready-made approach.
Not a near-term priority — track as a future option.
