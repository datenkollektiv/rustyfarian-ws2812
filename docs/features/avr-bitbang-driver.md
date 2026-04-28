# Feature: Bit-Banged AVR WS2812 Backend

*Status: Implemented (2026-05-04)*
*Created: 2026-05-04.*

## Motivation

`rustyfarian-avr-ws2812` currently drives WS2812 LEDs by streaming a prerendered byte sequence over the AVR's hardware SPI peripheral at 2 MHz (4 SPI bits per WS2812 bit).
Hardware bring-up on 2026-05-04 against an ATmega328P (both a CH340 Nano clone and a genuine Arduino Nano) revealed that this approach fails on the test strip even though the same strip runs cleanly with the ESP32 RMT drivers.

Symptoms:

- Stable white-ish output regardless of intended color, no flicker, brightness scaling proportional.
- With `NUM_LEDS = 1` (data for one LED only), LEDs 2 and 3 in the chain still flicker faintly — proof that LED 1 is not reliably consuming exactly 24 bits and partial data is leaking through the chain.

Root cause analysis (recorded in [`docs/project-lore.md`](../project-lore.md) under "AVR WS2812 Driver: SPI Prerendered Encoding Limitation"):

- The 2 MHz / 4-bit encoding emits `T0H = 500 ns` (right at the WS2812B "0/1" decision threshold) and `T1H = 1500 ns` (well above the 0.85 µs nominal max). Both rely on chip tolerance that varies between strip variants.
- The `ws2812-spi` README documents this exact symptom: *"Is everything white? This may stem from an SPI peripheral that's too slow or one that takes too much time in-between bytes."*
- The `arduino-hal` `SpiBus::write` polling loop introduces inter-byte gaps that consume a meaningful fraction of bit time at 2 MHz.

External research (saved to [`docs/research-avr-ws2812-driver-options.md`](../research-avr-ws2812-driver-options.md)) found:

- No public Rust WS2812 driver for AVR has hardware-verified working examples.
- `Adafruit_NeoPixel` and `FastLED` (in C) achieve reliability on the same hardware via cycle-counted inline assembly with global interrupts disabled — proven on millions of Arduino projects.
- This is the canonical working approach; an equivalent in Rust via `asm!` is technically open.

## Goals

1. Provide a second backend in `rustyfarian-avr-ws2812` that produces clean, in-spec WS2812 timing on ATmega328P at 16 MHz F_CPU.
2. Keep the existing SPI prerendered backend available for users on tolerant strips (it has no overhead beyond the byte stream and works fine for many).
3. Match the public API style of the SPI backend so users can swap with minimal code changes: `Ws2812BitBang::new(pin)` + `write(&[RGB8])`.
4. Document the trade-off so users can make an informed choice (interrupts disabled during write vs. SPI's freedom to interleave with other peripherals).

## Non-Goals

- Supporting MCUs other than ATmega328P at 16 MHz in this iteration — generalising the asm to other AVR clock rates or chips can come later.
- Replacing the SPI backend. Both stay; bit-bang is the recommended-default for Arduino Nano/Uno once verified.
- DMA-driven implementations.
- `embedded-hal` async support.

## Design Sketch

### Crate organisation

Same crate, new feature flag:

```toml
[features]
default = ["spi"]
spi = []        # current SPI prerendered backend
bitbang = []    # new cycle-counted asm backend
```

Public API:

```rust
#[cfg(feature = "spi")]
pub struct Ws2812Spi<SPI, const N: usize> { /* existing */ }

#[cfg(feature = "bitbang")]
pub struct Ws2812BitBang { /* new */ }
```

Both backends share `ws2812-pure::rgb_to_grb` for color conversion.
The bit-bang backend does *not* use `ws2812-pure::prerender_spi` (which is SPI-specific); it streams 24 bits per LED directly to a port pin.

### Timing budget at 16 MHz F_CPU

|            | Cycles | Time    | WS2812B spec |    Status     |
|:-----------|-------:|:--------|:-------------|:-------------:|
| T0H        |      4 | 250 ns  | 0.25–0.55 µs |   mid-spec    |
| T0L        |     16 | 1000 ns | 0.7–1.0 µs   |  upper-spec   |
| T1H        |      8 | 500 ns  | 0.65–0.95 µs |   mid-spec    |
| T1L        |     12 | 750 ns  | 0.30–0.60 µs | slightly over |
| Bit period |     20 | 1.25 µs | nominal      |     exact     |

These timings match `Adafruit_NeoPixel`'s published values for ATmega328P @ 16 MHz.

### API sketch

```rust
pub struct Ws2812BitBang {
    port: *mut u8,  // PORTx register
    pin_mask: u8,   // 1 << pin number
}

impl Ws2812BitBang {
    /// SAFETY: `port` must be a valid AVR PORT register, and `pin` must be configured as output by the caller.
    pub unsafe fn new(port: *mut u8, pin: u8) -> Self {
        Self { port, pin_mask: 1 << pin }
    }

    pub fn write(&mut self, colors: &[RGB8]) {
        avr_device::interrupt::free(|_| {
            for &color in colors {
                let grb = ((color.g as u32) << 16) | ((color.r as u32) << 8) | color.b as u32;
                unsafe { send_24_bits_asm(grb, self.port, self.pin_mask) };
            }
            // Pin remains low after exit; ≥50 µs reset latches before next call.
        });
    }
}
```

The const-generic-over-port alternative was considered but rejected for the first version: `arduino-hal`'s pin types don't expose static port-register addresses through const generics ergonomically.

### `send_24_bits_asm`

Implementation strategy:

1. Initialise: load the 24-bit `grb` value into three registers (or push to stack).
2. For each of 24 bits MSB-first:
   - Set port pin high (`out` instruction, 1 cycle).
   - Branch on bit value: emit T0H or T1H delay (4 vs 8 cycles).
   - Set port pin low (`out`, 1 cycle).
   - Pad remaining cycles to hit the total bit period of 20.
3. Loop overhead must be balanced — both branches of the bit-decision must produce identical total cycle counts.

Reference: `Adafruit_NeoPixel/esp.cpp` and `Adafruit_NeoPixel.cpp` (`#elif defined(__AVR__) ... #if F_CPU == 16000000UL ...`).
The published asm there is BSD-licensed and serves as a reference; we will write our own from spec but cross-check against it.

## Validation Plan

1. **Bench bit-stream test** — `#[cfg(test)]` software simulator in the crate that walks the bit-decision logic for known inputs and confirms the byte ordering and per-bit value selection match expected output. Cannot validate timing on host but rules out logic bugs.
2. **`cargo check --target avr-none --features bitbang`** — confirms the asm assembles cleanly on the project's pinned `nightly-2025-04-27`.
3. **Hardware validation on Arduino Nano** with the same WS2812 strip that fails today's SPI backend:
   - Pure red `PulseEffect` shows red (not white) across all LEDs.
   - `NUM_LEDS = 1` causes only LED 1 to light; LEDs 2 and 3 stay dark — proves chain alignment.
   - Rainbow effect cycles correctly with all hue transitions visible.
   - 30+ minute soak test — no drift, no flicker, no crashes.
4. **Long-strip stress test** — 30+ LEDs to confirm the global-interrupts-off window doesn't break ravedude UART or `delay_ms` measurably for typical animation update rates.

## Open Questions

- ~~**`asm!` macro reliability on AVR Tier 3 target**~~ — **Resolved 2026-05-04.** `asm!` inside functions compiles cleanly on `nightly-2025-04-27` for `avr-none` once `#![feature(asm_experimental_arch)]` is enabled. The `global_asm!` bug ([rust-lang/rust #134758](https://github.com/rust-lang/rust/issues/134758)) does not affect us. First spike (`examples/avr-nano-rainbow/src/bin/bitbang_spike.rs`) builds successfully against the pinned nightly.
- **Pin abstraction** — runtime port pointer is the proposed first version. Could be improved to const generics or a builder once the asm is proven, but only if it doesn't compromise timing.
- **F_CPU support beyond 16 MHz** — the initial version is 16 MHz only. Other clock rates (8 MHz, 20 MHz) are a follow-up. Document the limitation up front.
- **Interaction with arduino-hal pin types** — the user typically obtains a pin via `pins.d11.into_output()`. We need either a way to extract the port register address from that, or accept the user passing raw addresses (less ergonomic). Investigate whether `arduino-hal`'s pin types expose underlying port info through any public API.
- **Should the bit-bang backend become the default feature?** Recommend yes, *after* hardware validation — it works on more strips with no downside for strips the SPI variant would also handle.

## Alternatives Considered

| Alternative                                                | Why not (now)                                                                                                                                |
|:-----------------------------------------------------------|:---------------------------------------------------------------------------------------------------------------------------------------------|
| Track A: increase SPI prescaler to 4 MHz (one-line change) | Worth trying as a 5-minute experiment; if it works on this strip, defer Track B. Documented as Step 1 in the [roadmap entry](../ROADMAP.md). |
| Custom 3-bits-per-WS2812-bit SPI encoding at 2.4 MHz       | ATmega328P SPI prescaler doesn't produce 2.4 MHz cleanly; encoding doesn't align to byte boundaries.                                         |
| Custom 8-bits-per-WS2812-bit at 8 MHz SPI                  | At 8 MHz SPI the inter-byte gap consumes ~50% of bit time on AVR; even worse than current.                                                   |
| Move to a different chip family (e.g. RP2040)              | Out of scope — the project already targets AVR explicitly.                                                                                   |
| Drop AVR support                                           | Out of scope — the AVR backend is part of the triple-HAL strategy.                                                                           |

## Outcome Tracking

### Track A — 4 MHz SPI experiment (executed 2026-05-04)

**Result: Failed in a new way; informative for the ADR.**
Switched `OscfOver8` → `OscfOver4` in the example and reflashed against the same WS2812 strip + Arduino Nano that white-screened at 2 MHz.

Observation with `NUM_LEDS = 1` and `RainbowEffect`:
After a brief noisy startup, the strip settled into a *stable* pattern of 5 white LEDs + 1 green LED — six LEDs lit when only one should be.

Diagnostic interpretation:
- The appearance of **green** (rather than only white) confirms the strip can now distinguish "0" from "1" bits — the lower `T0H = 250 ns` (vs the borderline 500 ns at 2 MHz) is correctly recognised as a "0".
- The chain misalignment (six lit instead of one) is *worse* than at 2 MHz, because the bit period is now 1.0 µs (vs WS2812 nominal 1.25 µs) and the inter-byte gap from `arduino-hal`'s `raw_transaction` polling loop (~5–15 CPU cycles ≈ 0.3–1.0 µs) is now comparable to a single WS2812 bit time. The strip's bit counter desyncs at every byte boundary.

Conclusion:
- 2 MHz: bit decoding borderline (everything reads as "1") → bit count usually correct, colors usually wrong.
- 4 MHz: bit decoding reliable, but inter-byte gaps eat the bit period → bit count wrong, fragments leak into chain.

No SPI rate available on the ATmega328P prescaler ladder simultaneously satisfies both constraints. Track A is closed as **not viable**.

Reverted prescaler to `OscfOver8` so the example doesn't ship in a half-broken state.

### Track B — Bit-bang (spike validated 2026-05-04, full pipeline 2026-05-04, **production driver landed 2026-05-04**)

First spike `examples/avr-nano-rainbow/src/bin/bitbang_spike.rs` produced a **pulsing red strip** on the same hardware that the SPI driver couldn't drive correctly.
Three spike-level checkpoints resolved positively:

- ✅ **`asm!` macro reliability** — compiles cleanly on `nightly-2025-04-27` for `avr-none` with `#![feature(asm_experimental_arch)]`.
- ✅ **Cycle-counted timing on real hardware** — colors render as red (not white-ish), no chain leakage, animation visibly updates frame-to-frame.
- ✅ **Full effect-pipeline integration** — swapped the hand-rolled triangle ramp for `ferriswheel::PulseEffect` driving the same asm send routine; result is a smooth sine-curve red breath across all LEDs. Confirms `&[RGB8]` from any `Effect` feeds the bit-bang send routine cleanly.

All planned steps complete:

1. ✅ **Generalised the asm into `rustyfarian-avr-ws2812`** as `Ws2812BitBang<P, const PORT_ADDR: u8, const PIN_BIT: u8>` behind the `bitbang` cargo feature. Const-generic `asm!` operands (the highest-risk open question) compiled cleanly on `nightly-2025-04-27` — no fallback to a `PinDescriptor` trait was needed. Driver owns the configured output pin and wraps `write` in `avr_device::interrupt::free` internally.
2. ✅ **Hardware-validated the production driver path** with `examples/avr-nano-rainbow/src/bin/bitbang_demo.rs` driving `ferriswheel::PulseEffect` — identical visible behaviour to the spike.
3. ✅ **`SmartLedsWrite` impl for both backends** (feature `smart-leds-trait`) for sister-driver parity.
4. ✅ **ADR 007 updated** with the implementation note recording the final shape.
5. ✅ **Lore entry resolved** in [`docs/project-lore.md`](../project-lore.md).

Out of scope (deferred follow-ups, not blocking this feature):

- Generalising to other AVR clock rates (currently 16 MHz only).
- Generalising to ports outside the low I/O space (PORTE+ on Mega2560).
- Optional Adafruit-style "head20" single-block asm to tighten the per-bit slack budget.
