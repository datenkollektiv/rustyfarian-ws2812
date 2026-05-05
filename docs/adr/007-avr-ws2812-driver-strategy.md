# ADR 007: AVR WS2812 Driver Strategy — Bit-Bang as Default, SPI as Opt-In

## Status

Accepted (2026-05-04). Production driver landed and hardware-validated 2026-05-04 — see [Implementation note](#implementation-note) below.

## Context

ADR 005 established the triple-HAL strategy that includes `rustyfarian-avr-ws2812` for ATmega328P (AVR) targets.
The initial AVR backend was implemented in March 2026 using **SPI prerendered encoding** — the same approach used by the `ws2812-spi` crate's prerendered module:

- 2 MHz SPI clock (`OscfOver8` on a 16 MHz `F_CPU`)
- 4 SPI bits per WS2812 bit, packed two-WS2812-bits per SPI byte
- Encoding patterns `0x88 / 0x8E / 0xE8 / 0xEE` for the four `00 / 01 / 10 / 11` cases
- 12 SPI bytes per LED + 80 trailing zero bytes for the > 50 µs reset

This produces nominal timings:

| Bit | T_H     | T_L     | Spec (WS2812B nominal)     |
|:----|:--------|:--------|:---------------------------|
| 0   | 500 ns  | 1500 ns | T0H ≤ 550 ns, T0L ≥ 700 ns |
| 1   | 1500 ns | 500 ns  | T1H ≈ 700 ns, T1L ≈ 600 ns |

`T0H = 500 ns` is right at the WS2812B "0/1" decision threshold; `T1H = 1500 ns` is well above the nominal max.
Both rely on chip tolerance, which varies between WS2812 / WS2812B / clone variants.

Hardware bring-up on 2026-05-04 against an ATmega328P (both a CH340 Nano clone and a genuine Arduino Nano) with a WS2812 strip that runs cleanly on `rustyfarian-esp-idf-ws2812` and `rustyfarian-esp-hal-ws2812` revealed the SPI prerendered backend produces stable white-ish output (no flicker, brightness scaling proportional, but every channel appears similarly lit).
A `NUM_LEDS = 1` test exposed chain-leakage to LEDs 2 and 3 — mechanical proof that LED 1 is not reliably consuming exactly 24 bits.

Diagnostic dead-ends ruled out: crystal frequency mismatch, GRB color order, strip variant (works on ESP), `PulseEffect` math, USB power supply, cable length, and Arduino board (clone vs genuine).
Full root-cause record in [`docs/project-lore.md`](../project-lore.md) "AVR WS2812 Driver: SPI Prerendered Encoding Limitation."

External research ([`docs/research-avr-ws2812-driver-options.md`](../research-avr-ws2812-driver-options.md)) found:

- No public Rust WS2812 driver for AVR has hardware-verified working examples.
- The `ws2812-spi` README explicitly documents this exact symptom: *"Is everything white? This may stem from an SPI peripheral that's too slow or one that takes too much time in-between bytes."*
- `Adafruit_NeoPixel` and `FastLED` (in C, used by millions of Arduino projects) achieve reliability on the same hardware via cycle-counted inline assembly with global interrupts disabled.

Two driver paths were explored on 2026-05-04 to confirm before deciding:

### Track A — Faster SPI (4 MHz, `OscfOver4`)

A one-line change to push `T0H` from 500 ns to 250 ns (mid-spec) and `T1H` from 1500 ns to 750 ns (in-spec).

**Result:** failed in a new way — the strip settled into a stable "5 white + 1 green" pattern, indicating bit decoding is now reliable but chain alignment is broken.
At 4 MHz, the bit period drops to 1.0 µs (vs WS2812 nominal 1.25 µs); the inter-byte gap from `arduino-hal`'s polling-based `raw_transaction` loop (~0.3–1.0 µs) is comparable to or larger than a single WS2812 bit time, so the strip's bit counter desyncs at every byte boundary.

| Prescaler    | Bit decoding        | Chain alignment                         |
|:-------------|:--------------------|:----------------------------------------|
| `/8` (2 MHz) | Borderline (white)  | OK (within tolerance)                   |
| `/4` (4 MHz) | OK (color visible)  | Broken (inter-byte gap eats bit period) |

No SPI rate available on the ATmega328P prescaler ladder simultaneously satisfies both constraints.
Higher rates (8 MHz) make the inter-byte-gap problem proportionally worse.

### Track B — Cycle-counted bit-bang via `asm!`

A spike (`examples/avr-nano-rainbow/src/bin/bitbang_spike.rs`) using inline `sbi 0x05, 3` / `cbi 0x05, 3` instructions on PB3 (Arduino D11) with cycle-counted nop padding.
Adapted from `Adafruit_NeoPixel`'s proven ATmega328P @ 16 MHz timing:

| Bit | T_H       | T_L       | Total     |
|:----|:----------|:----------|:----------|
| 0   | 4 cycles  | 16 cycles | 20 cycles |
| 1   | 13 cycles | 7 cycles  | 20 cycles |

**Result:** smooth red breath via `ferriswheel::PulseEffect` rendered correctly across all LEDs.
End-to-end pipeline validated — `&[RGB8]` from any `Effect` feeds the asm send routine cleanly.

Three checkpoints resolved positively:

- ✅ `asm!` macro compiles on `nightly-2025-04-27` for `avr-none` once `#![feature(asm_experimental_arch)]` is enabled.
- ✅ Cycle-counted bit-bang produces correct WS2812 timing on real hardware (no white-ish output, no chain leakage).
- ✅ Full effect-pipeline integration (`PulseEffect` → asm-send) renders as expected.

## Decision

Adopt **cycle-counted inline-`asm!` bit-bang** as the default AVR WS2812 backend in `rustyfarian-avr-ws2812`, while retaining the SPI prerendered backend as an opt-in alternative.

### Implementation outline

The `rustyfarian-avr-ws2812` crate adds a `bitbang` cargo feature (default-on in a future minor release; initially opt-in alongside `spi` until the API surface stabilises):

```toml
[features]
default = ["spi"]
spi = []
bitbang = []
```

Public types:

```rust
#[cfg(feature = "spi")]
pub struct Ws2812Spi<SPI, const N: usize> { /* unchanged */ }

#[cfg(feature = "bitbang")]
pub struct Ws2812BitBang { /* runtime port pointer + pin mask */ }
```

Both backends share `ws2812-pure::rgb_to_grb` for color conversion.
The bit-bang backend does not use `ws2812-pure::prerender_spi` (which is SPI-specific); it streams 24 bits per LED directly to a port pin via inline assembly.

The `Ws2812BitBang::write` method wraps the asm in `avr_device::interrupt::free(..)` so global interrupts are disabled for the entire frame — the standard tradeoff used by `Adafruit_NeoPixel` and `FastLED`.

### Constraints (initial release)

- ATmega328P at 16 MHz `F_CPU` only. Other AVR variants and clock rates are a follow-up.
- Initial pin support is limited to PORTB pins on the ATmega328P (where `sbi`/`cbi` work directly). General-port support via `st`-with-pointer-register (the Adafruit "head20" pattern) is a clean follow-up.

### Why retain the SPI backend

- Some users have tolerant strip variants on which SPI prerendered works fine. They benefit from being able to use other SPI peripherals concurrently.
- Removing the SPI backend is a breaking change for any consumer that already depends on the v0.3 / v0.4 API.
- Documenting the trade-off explicitly is more honest than silently switching: SPI = smaller code, free interrupt servicing, less reliable. Bit-bang = larger code, blocks interrupts during write, mechanically reliable.

## Consequences

### Positive

- **Mechanically correct WS2812 timing** — in-spec `T_H` and `T_L` for both 0 and 1 bits, no chip-tolerance dependency.
- **Proven approach** — matches the timing pattern shipped in millions of Arduino projects via `Adafruit_NeoPixel` / `FastLED`.
- **Hardware-validated** — the spike rendered correctly on the strip that the SPI backend could not drive.
- **Non-breaking** — the SPI backend stays usable; users opt in to the new feature flag.
- **Pure-logic crates unaffected** — `ws2812-pure`, `ferriswheel`, and `led-effects` need no changes; the bit-bang backend reuses `rgb_to_grb` and consumes `&[RGB8]` like everything else.

### Negative

- **Nightly required** — the AVR target is already Tier 3 (nightly-only); now also requires `#![feature(asm_experimental_arch)]`. Documented in `docs/avr-getting-started.md`.
- **Global interrupts disabled per frame** — at 1.25 µs/bit × 24 bits/LED × N LEDs, the disabled-interrupt window scales linearly. For 100 LEDs this is 3 ms per frame; `millis()` and serial UART will drop ticks during the write window. Standard tradeoff; documented.
- **F_CPU coupling** — the initial cycle counts are valid only at 16 MHz. Other clock rates need separate cycle-counted variants (or a runtime divider, which costs cycles).
- **Pin constraint** (initial release) — only PORTB pins on ATmega328P. General-port support is planned but adds asm complexity.
- **Two backends to maintain** — mitigated by isolating each behind a feature flag and sharing the GRB conversion helper.

### Neutral

- **Existing code stays at parity** — the SPI backend remains buildable and tested; no migration is forced on users for whom it works.
- **Future expansion path is clear** — the asm pattern is well-understood (Adafruit's "head20" for any-port support), so generalisation is incremental rather than exploratory.

## Implementation note

The production `Ws2812BitBang` driver landed in `crates/rustyfarian-avr-ws2812/` on 2026-05-04 behind the `bitbang` cargo feature, alongside the unchanged SPI backend.

Final shape, validated against the same Arduino Nano + WS2812 strip the SPI backend couldn't drive correctly:

- `Ws2812BitBang<P, const PORT_ADDR: u8, const PIN_BIT: u8>` — const generics over the AVR port-register address and pin bit. The asm uses `const` operands (`asm!("sbi {p}, {n}", p = const PORT_ADDR, n = const PIN_BIT, ...)`), keeping the `sbi`/`cbi` 2-cycle path and matching the spike's hardware-validated cycle counts.
- `ports::{PORTB, PORTC, PORTD}` constants for the three ATmega328P GPIO ports in the low I/O space.
- The driver owns the configured output pin so DDR is tied to the driver's lifetime, mirroring `Ws2812Spi`'s ownership of `SPI`.
- `write` wraps the asm loop in `avr_device::interrupt::free(..)` internally — timing is mandatory, not opportunistic.
- `SmartLedsWrite` adapter implemented for both backends behind `feature = "smart-leds-trait"` for sister-driver parity.
- The original spike (`examples/avr-nano-rainbow/src/bin/bitbang_spike.rs`) is retained as a low-level reference; the new `bin/bitbang_demo.rs` is the recommended example using the production driver.

The first hardware-test of the production driver ran `ferriswheel::PulseEffect` end-to-end and matched the spike's visible behaviour exactly: smooth red breath, no flicker, no chain leakage. The const-generic `asm!` approach (the "highest-risk open question" in the feature doc) compiled cleanly on the project's pinned `nightly-2025-04-27` once `#![feature(asm_experimental_arch)]` was added — no fallback to a `PinDescriptor` trait was needed.

Open work tracked separately, not part of this ADR's accepted scope:

- Generalising to other AVR clock rates (currently 16 MHz only).
- Generalising to ports outside the low I/O space (PORTE+ on Mega2560).
- Optional Adafruit-style "head20" single-block asm to tighten the per-bit slack budget.

## References

- [`docs/features/archive/avr-bitbang-driver.md`](../features/archive/avr-bitbang-driver.md) — full design + Track A / Track B experiment records.
- [`docs/project-lore.md`](../project-lore.md) — "AVR WS2812 Driver: SPI Prerendered Encoding Limitation" lore entry.
- [`docs/research-avr-ws2812-driver-options.md`](../research-avr-ws2812-driver-options.md) — external ecosystem research.
- [`docs/ROADMAP.md`](../ROADMAP.md) — "Reliable AVR WS2812 backend" entry.
- ADR 005 — "Dual-HAL Strategy" (now triple-HAL with the AVR addition).
- `Adafruit_NeoPixel.cpp` — reference cycle-counted asm for ATmega328P @ 16 MHz (BSD-licensed).
- [rust-lang/rust #134758](https://github.com/rust-lang/rust/issues/134758) — `global_asm!` bug on AVR; does not affect this decision because we use `asm!` inside functions.
