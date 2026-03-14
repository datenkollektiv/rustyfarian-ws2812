# AVR WS2812 Driver Options — Research Findings

*Research date: 2026-05-04*

## Executive Summary

- No Rust WS2812 driver for AVR has publicly documented hardware-verified success (photos, videos, user field reports) on an ATmega328P Arduino Nano/Uno.
- The `ws2812-spi` crate (v0.5.1, June 2025) explicitly names "all white output" as the diagnostic for SPI inter-byte timing failures; its own README singles out AVR as too slow for the non-prerendered variant — and the prerendered variant still requires zero inter-byte gaps, which is hard to guarantee in software on an 8-bit MCU.
- The **4-bit prerendered SPI encoding** (our current approach: `0b1000`/`0b1110` at 2 MHz) produces T0H ≈ 500 ns, which sits exactly at the WS2812 datasheet maximum; strips with tighter or shifted tolerances will interpret it as T1H, producing all-white or wrong-color output.
- One pure-Rust AVR clockless/bit-bang driver exists (`devcexx/ws2812-avr`), but it is a hobby project with 1 star, no releases, and no hardware-verification record; it also requires an unstable nightly feature (`generic_const_exprs`).
- The definitive working solution for AVR WS2812 in any language is **cycle-counted inline assembly bit-bang**, as used by Adafruit NeoPixel and FastLED; no production-quality Rust wrapper for ATmega328P exists yet — this is an open niche.

---

## Per-Item Analysis

<details>
<summary><strong>ws2812-spi (smart-leds-rs ecosystem)</strong></summary>

**Purpose:**
`embedded-hal` SPI-based WS2812 driver for the smart-leds ecosystem.
Provides two variants: a streaming variant (real-time CPU renders SPI bytes) and a prerendered variant (full buffer pre-computed before DMA/SPI send).

**Architecture:**
Pure-logic prerendering is in `src/prerendered.rs`; the streaming encoder generates bytes in an iterator.
Both variants are `no_std`.
Logic is decoupled from any specific HAL — it relies only on `embedded-hal`'s `SpiBus` or `SpiDevice` trait.
Tests can be run on a host machine.

**AVR-specific guidance (from README and docs.rs):**
- Streaming variant is explicitly documented as unsuitable for AVR: "If your core is too slow (for example, the AVR family), you may want to use the prerendered variant."
- "Is everything white? This may stem from an spi peripheral that's too slow or one that takes too much time in-between bytes."
- No AVR usage examples exist in the repository.

**Encoding and timing:**
- 4 SPI bits per WS2812 bit at 2–3.8 MHz.
- At 2 MHz: T0H = 500 ns (1 high bit × 500 ns/bit), T1H = 1000 ns (2 high bits).
- At 2.5 MHz: T0H = 400 ns, T1H = 800 ns (closer to datasheet center).
- At 3.8 MHz: T0H ≈ 263 ns, T1H ≈ 526 ns (within spec).
- Inter-byte gaps on AVR SPI are caused by CPU software-loading the next byte into `SPDR` after the previous byte finishes; the WS2812 requires the gap to be less than the T0L minimum (~450 ns) or it may trigger an early latch.

**Test coverage:**
Unit tests present; host-runnable.
No embedded integration tests.

**Maintenance status:**
v0.5.1, June 3, 2025.
Active — 3 open issues (none are AVR timing reports).

**Key open issues:**
- `#41` (Mar 2025): "Works on SPI0 on an RPI, fails on SPI1" — cross-peripheral timing variation.
- `#7` (Dec 2019): RGB vs. GRB order ambiguity.
- No filed issues specifically about AVR white output or chain misalignment.

**What we can learn:**
The "all white" symptom is canonical documentation for inter-byte gap or T0H boundary violation.
The crate's prerendered module is structurally identical to our `rustyfarian-avr-ws2812` approach.

</details>

<details>
<summary><strong>devcexx/ws2812-avr (clockless/bit-bang, pure Rust)</strong></summary>

**Purpose:**
A clockless WS2812 driver for AVR microcontrollers, written in pure Rust.
Described by its author as "done for fun as a part of a Rust learning process."

**Architecture:**
Clockless — does not use hardware SPI.
Relies on `avr-hal` for GPIO pin access.
Processor selected via Cargo feature flags matching `avr-hal` processor names.
Requires `#![feature(generic_const_exprs)]` — an unstable nightly-only feature.

**Timing strategy:**
Not described in public-facing documentation.
Implementation details are in library docstrings and the `examples/` directory (source not publicly accessible via web fetch at research time).
Presumably uses `avr_device::interrupt::free()` to disable interrupts during bit output.

**Hardware verification:**
None documented.
No photos, videos, or user field reports found.
Author describes it as a learning project.

**Maintenance status:**
1 star, 3 forks, 9 commits.
No published crates.io releases.
Tested against nightly-2024-04-15.
No commits found post-2024.

**Risks:**
`generic_const_exprs` is a long-lived unstable feature with no stabilization timeline.
Our workspace pins `nightly-2025-04-27`; compatibility with a library requiring nightly-2024-04-15 would need verification.
No evidence of production or even hobbyist hardware use.

**What we can learn:**
The existence of this project confirms the community recognizes SPI-prerendered as insufficient for AVR.
The clockless/bit-bang direction is the right architectural alternative.

</details>

<details>
<summary><strong>avr-hal examples (Rahix)</strong></summary>

**Purpose:**
Reference HAL and examples for AVR-based boards in Rust.
Covers Arduino Uno, Nano, Mega, Leonardo, SparkFun Pro Micro, Trinket, and others.

**WS2812/NeoPixel coverage:**
No WS2812, NeoPixel, or smart-leds example exists in `examples/arduino-uno/src/bin/` (21 examples verified).
The Nano, Mega, and other board directories were not individually traversed but the avr-hal README does not advertise any LED-strip example.

**SPI example:**
`uno-spi-feedback.rs` is present — a loopback test, not WS2812-related.

**What this means:**
The most widely used Rust AVR framework has no WS2812 example.
This is a gap that `rustyfarian-avr-ws2812` is positioned to fill — even if it currently fails on hardware.

</details>

<details>
<summary><strong>ws2812-rs (no_std WS2812B driver)</strong></summary>

**Purpose:**
A lightweight `no_std` WS2812B driver using configurable delay-trait-based timing strategies.

**Architecture:**
`no_std`, platform-agnostic.
Relies on `embedded-hal` delay traits for timing rather than SPI or bit-bang assembly.

**AVR suitability:**
Delay-trait-based timing on AVR is unreliable at WS2812 precision levels: each delay call involves function call overhead and variable instruction counts.
Not documented as AVR-verified.
No hardware reports found.

**Maintenance status:**
Listed on lib.rs; exact version and last commit not retrieved.

</details>

<details>
<summary><strong>Adafruit NeoPixel / FastLED (C reference implementations)</strong></summary>

**Purpose:**
These are the de-facto standard WS2812 drivers for AVR Arduino boards.
Not Rust, but the definitive reference for what actually works on ATmega328P hardware.

**Encoding strategy:**
Both use **cycle-counted inline assembly bit-bang**, with platform-specific ASM blocks selected at compile time based on `F_CPU`.
For 16 MHz ATmega328P:
- T0H is generated by holding the pin high for exactly 3–4 cycles (188–250 ns), well below the 500 ns maximum.
- T1H is generated by holding high for 7–8 cycles (437–500 ns).
- Total bit period is ~1250 ns (800 kHz data rate).
- Interrupts are globally disabled (`cli`) for the entire frame transfer.

**Why this works where SPI-prerendered fails:**
Bit-bang produces T0H of ~200–350 ns (centered in spec).
SPI 4-bit encoding at 2 MHz produces T0H of exactly 500 ns (datasheet maximum; some chips reject it).
Bit-bang avoids inter-byte gaps entirely.

**Hardware verification:**
Decades of verified hardware use across millions of Arduino units.

</details>

---

## Comparison Table

|                                     Approach | Rust?  | AVR support            | T0H (ns)       | T1H (ns)          | Inter-byte gap     | HW-verified on Nano/Uno | Maintenance         |
|---------------------------------------------:|:------:|:-----------------------|:---------------|:------------------|:-------------------|:------------------------|:--------------------|
| ws2812-spi prerendered @ 2 MHz (our current) |  Yes   | Documented but brittle | 500 (at limit) | 1000 (above spec) | Possible (SW load) | No reports              | v0.5.1, Jun 2025    |
|             ws2812-spi prerendered @ 3.8 MHz |  Yes   | Possible               | 263            | 526               | Possible (SW load) | No reports              | v0.5.1, Jun 2025    |
|          devcexx/ws2812-avr (clockless Rust) |  Yes   | Claimed                | Unknown        | Unknown           | N/A (bit-bang)     | None documented         | 1 star, no release  |
|          Adafruit NeoPixel / FastLED (C ASM) |   No   | Yes (definitive)       | ~200–350       | ~437–500          | N/A (disabled IRQ) | Millions of boards      | Actively maintained |
|     Rust inline-asm bit-bang (not yet built) |  Yes   | Would be               | ~200–350       | ~437–500          | N/A (disabled IRQ) | Not yet                 | Not yet             |

---

## Root Cause Analysis of Our Observed Failure

The symptoms we observed are consistent with a T0H boundary violation:

1. **"Stable white-ish output"** — every bit is decoded as `1` because T0H (500 ns) equals or exceeds the WS2812 decision threshold on this strip variant.
2. **"Brightness scaling visibly proportional"** — brightness is applied uniformly to `(1, 1, 1)` white, so dimming works but hue does not.
3. **"LEDs 2 and 3 flicker with NUM_LEDS = 1"** — LED 1 is consuming 24 bits of ones (all-white), passing through exactly 24 bits, but the inter-byte gaps from SPI SW-loading may be causing the WS2812 to occasionally latch early or late, leaking bits into LED 2's slot.

The josh.com empirical study shows the WS2812 latch threshold is approximately 6 µs of low signal, not 50 µs as in the datasheet.
An AVR SPI inter-byte gap — the time between the last bit of one byte clocking out and the first bit of the next byte starting — depends on how quickly the CPU can write to `SPDR` after the `SPIF` flag is set.
At 16 MHz, even a tight polling loop takes ~4–8 cycles (250–500 ns) in between bytes; a prerendered buffer passed to an AVR HAL SPI `write_all` call introduces additional overhead per byte.
If consecutive bytes happen to create low periods longer than 6 µs at a bit boundary, an early latch occurs — explaining partial chain pass-through.

**Encoding frequency sensitivity:**
- At 2 MHz (our current): T0H = 500 ns = datasheet maximum — any strip that is slightly tighter fails.
- At 2.5 MHz: T0H = 400 ns — centered in spec, better tolerance.
- At 3.8 MHz: T0H = 263 ns — within spec, but SPI prescaler options on ATmega328P at 16 MHz are limited to: 125 kHz, 250 kHz, 500 kHz, 1 MHz, 2 MHz, 4 MHz, 8 MHz, 16 MHz — there is no 2.5 MHz or 3.8 MHz option.
- The nearest usable frequencies are 2 MHz (prescaler /8) and 4 MHz (prescaler /4).
- At 4 MHz: T0H = 250 ns (1 bit × 250 ns), T1H = 500 ns (2 bits × 250 ns) — T1H is at the datasheet maximum but T0H is well within spec.

---

## Encoding Alternative: Inline Assembly Bit-Bang

The C ecosystem (Adafruit NeoPixel, FastLED) drives WS2812 reliably on ATmega328P using cycle-counted inline assembly.
The approach:

1. Globally disable interrupts before the frame (`cli` instruction, ~1 cycle).
2. For each bit, in a tight ASM loop:
   - Drive the pin HIGH.
   - Count cycles for T0H (~4 cycles = 250 ns) or T1H (~8 cycles = 500 ns).
   - Drive the pin LOW.
   - Count remaining cycles to fill the 20-cycle total bit period (1250 ns at 16 MHz).
3. After all bits, restore interrupts (`sei`).
4. Hold the pin low for ≥ 6 µs (≥ 96 cycles) to latch.

**Rust feasibility:**
Rust's `asm!` macro (stable since Rust 1.59) supports AVR inline assembly on nightly.
The AVR `asm!` macro works in nightly builds; we already use `nightly-2025-04-27`.
A known compiler issue (`rust-lang/rust #134758`, filed Dec 2024) affects `global_asm!` in `lib.rs` on AVR — `asm!` inside function bodies is not affected by this bug.
The Adafruit 16 MHz ATmega328P ASM loop is about 15–20 instructions and could be directly ported to Rust `asm!` syntax.

**Key implementation constraints:**
- The pin must be a compile-time-known GPIO register address for single-cycle `SBI`/`CBI` operations; the `avr-hal` GPIO abstraction may introduce overhead.
- Cycle counting requires knowing `F_CPU` at compile time; a `const` parameter or build-time assertion is sufficient.
- `avr_device::interrupt::free()` disables interrupts for a closure — appropriate for frame sends.

---

## Strategic Recommendation

**Short term — try 4 MHz SPI before abandoning the SPI approach:**
The current 2 MHz SPI rate produces T0H at the datasheet maximum.
Switching to 4 MHz (ATmega328P SPI prescaler /4) yields T0H = 250 ns and T1H = 500 ns, both well-centered in spec.
The 4-bit encoding pattern changes: at 4 MHz, the `0b1000` pattern sends T0H = 250 ns (1 bit high at 4 MHz).
This is a one-line change to the SPI clock configuration and worth testing before committing to bit-bang.
The inter-byte gap risk remains at 4 MHz, but the timing sensitivity to that gap is lower because the overall bit period is shorter.

**Medium term — implement inline-asm bit-bang as the authoritative AVR driver:**
The SPI-prerendered approach is structurally unreliable on AVR because inter-byte gaps are non-deterministic without hardware FIFO or DMA.
The only approach that is known to work reliably on ATmega328P is cycle-counted bit-bang with interrupts disabled.
A Rust `asm!`-based implementation porting the Adafruit NeoPixel 16 MHz AVR loop is feasible with our current nightly toolchain.
This should live in `rustyfarian-avr-ws2812` as an alternative backend, with the SPI backend retained for documentation purposes.

**What to document as "known limitation":**
Until a bit-bang backend is implemented and hardware-verified, `rustyfarian-avr-ws2812` should be marked in its README as "SPI-prerendered approach — works on strips with relaxed T0H tolerance; for guaranteed compatibility use the bit-bang backend (planned)."

**Regarding external crates:**
- `devcexx/ws2812-avr`: Monitor — the clockless direction is correct, but the crate is not production-ready (no releases, unstable features, no hardware reports).
Do not depend on it; use it as architecture reference only.
- `ws2812-spi`: Continue using for ESP32 targets only.
Document that its prerendered variant is insufficient for ATmega328P in our README.
- `avr-hal`: No WS2812 example gap to fill — this is an opportunity to contribute a working example upstream once our bit-bang driver is verified.

---

## Sources

- [GitHub — smart-leds-rs/ws2812-spi-rs](https://github.com/smart-leds-rs/ws2812-spi-rs)
- [ws2812-spi v0.5.1 — docs.rs](https://docs.rs/crate/ws2812-spi/latest)
- [ws2812-spi — crates.io](https://crates.io/crates/ws2812-spi)
- [GitHub — devcexx/ws2812-avr](https://github.com/devcexx/ws2812-avr)
- [GitHub — Rahix/avr-hal](https://github.com/Rahix/avr-hal)
- [avr-hal examples — arduino-uno/src/bin](https://github.com/Rahix/avr-hal/tree/main/examples/arduino-uno/src/bin)
- [GitHub topics: neopixel (Rust)](https://github.com/topics/neopixel?l=rust)
- [Bit Banging WS2812 in Rust — Hackster.io](https://www.hackster.io/dcaponi1/bit-banging-ws2812-in-rust-bb30bc)
- [smart-leds: WS2812 and similar LEDs with Rust — sawatzke.dev](https://sawatzke.dev/blog1/smartleds/)
- [NeoPixels Revealed: How to (not need to) generate precisely timed signals — josh.com](https://wp.josh.com/2014/05/13/ws2812-neopixels-are-not-so-finicky-once-you-get-to-know-them/)
- [Control WS2812 LEDs with STM32 SPI — ControllersTech](https://controllerstech.com/ws2812-leds-using-stm32-spi/)
- [WS2812 Driver — QMK Firmware docs](https://docs.qmk.fm/drivers/ws2812)
- [Compile Error on AVR atmega328p with global_asm! — rust-lang/rust #134758](https://github.com/rust-lang/rust/issues/134758)
- [megaTinyCore tinyNeoPixel.md — SpenceKonde](https://github.com/SpenceKonde/megaTinyCore/blob/master/megaavr/extras/tinyNeoPixel.md)
- [More RPI SPI/WS2812 problems — Hackaday.io](https://hackaday.io/project/19927-tron-identity-disc-upgrade1/log/58484-more-rpi-spiws2812-problems-and-usb)
- [ws2812-spi-rs open issues](https://github.com/smart-leds-rs/ws2812-spi-rs/issues)
