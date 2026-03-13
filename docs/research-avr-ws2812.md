# AVR WS2812 Rust Ecosystem Research

Feasibility assessment for adding a `rustyfarian-avr-ws2812` crate to the workspace,
covering the current AVR Rust toolchain, available crates, timing approaches,
and an assessment of what a new crate would need to provide.

## Executive Summary

- The Rust AVR toolchain is functional in 2025 but permanently requires a pinned nightly compiler and the GNU AVR toolchain (`avr-gcc`, `avr-binutils`, `avr-libc`) — there is no stable-Rust or LLVM-only path.
- Two viable timing approaches exist for WS2812 on AVR: SPI-based encoding via `ws2812-spi` (prerendered mode) and direct bitbang with cycle-counted assembly (`ws2812-avr`, `light_ws2812`); SPI is strongly preferred for a Rust crate because it avoids inline assembly and is more portable.
- The `ws2812-spi` crate (v0.5.1, June 2025) is the highest-quality existing option and supports AVR via its prerendered mode, but depends on `embedded-hal 0.2` / `smart-leds-trait 0.2` and does not separate pure logic from hardware.
- `avr-hal` completed its `embedded-hal 1.0` migration in January 2025 ([issue #77](https://github.com/Rahix/avr-hal/issues/77) closed), but `ws2812-spi` still targets `embedded-hal 0.2` — the two ecosystems are currently mismatched.
- A `rustyfarian-avr-ws2812` crate is technically feasible and architecturally justified, but the embedded-hal version mismatch and permanent nightly toolchain requirement are significant practical blockers that must be resolved before committing.

---

## Rust AVR Toolchain Status

<details>
<summary><strong>Target, tier, and GNU dependency</strong></summary>

The only built-in AVR target in upstream Rust is `avr-unknown-gnu-atmega328`.
It is a **Tier 3** target, meaning it receives no automated testing or build guarantees from the Rust project.
The target unconditionally requires `avr-gcc`, `avr-binutils`, and `avr-libc` from the GNU project.
There is no current path to compiling for AVR using LLVM's LLD alone; LLVM's LLD has limited experimental AVR support, but it is not complete enough for production use.

For other AVR chips (ATmega2560, ATtiny85, etc.), the built-in target specification JSON can be extracted and modified by changing the `cpu` field; there is no separate built-in target per chip.

Build invocations require `-Z build-std=core` and must be run with `cargo +nightly build --target avr-unknown-gnu-atmega328 --release`.
The `--release` flag is mandatory for WS2812 work — the timing of SPI byte transmission is sensitive to compiler optimisation level.

</details>

<details>
<summary><strong>Nightly requirements and stability history</strong></summary>

A nightly compiler has been required since AVR support was first added to upstream LLVM.
The `avr-hal` project manages this by providing a `rust-toolchain.toml` that pins to a specific tested nightly.

Notable milestones in 2024–2025:

| Toolchain | Status |
|:----------|:-------|
| `nightly-2024-04-15` | Minimum noted working for `ws2812-avr` |
| `nightly-2024-11-05` | Previous `avr-hal` pin |
| `nightly-2025-01-01` | Upgraded to include LLVM 20 fixes for out-of-range branch targets |
| `nightly-2025-02-13` | Invalid assembly generation issue resolved |
| `nightly-2025-04-27` | Current `avr-hal` `rust-toolchain.toml` pin (as of March 2026) |

The upgrade from `nightly-2024-11-05` to `nightly-2025-01-01` fixed two LLVM bugs
([`llvm-project#118015`](https://github.com/llvm/llvm-project/issues/118015) and [`llvm-project#121498`](https://github.com/llvm/llvm-project/issues/121498)) that caused out-of-range branch targets
on devices lacking the `jmp` instruction (notably ATtiny85).
These fixes landed in LLVM 20 and were integrated via [`rust-lang/rust#135763`](https://github.com/rust-lang/rust/pull/135763).

A separate open issue ([`rust-lang/rust#134758`](https://github.com/rust-lang/rust/issues/134758), December 2024) affects `global_asm!` in library
crates targeting AVR: the compiler fails to propagate `target-cpu` metadata to LLVM during
library compilation, causing "instruction requires a CPU feature not currently enabled" errors.
The workaround for WS2812 driver development is to set `lto = false` or structure timing-critical
assembly as binary-crate code rather than library code.
This issue has **moderate impact** on driver crates that use inline assembly.

</details>

---

## Existing WS2812 Crates for AVR / ATmega

<details>
<summary><strong>ws2812-spi (smart-leds-rs, v0.5.1, June 2025)</strong></summary>

**Purpose**

An `embedded-hal` SPI driver for WS2812 LEDs, part of the `smart-leds-rs` organisation.
Implements `SmartLedsWrite` from `smart-leds-trait`.

**Architecture**

A single-crate, hardware-oriented library.
No separation of pure color logic from hardware concerns.
No unit tests runnable on a host machine.
Depends on `embedded-hal 0.2.4` and `smart-leds-trait 0.2.x`.

**AVR support: prerendered mode**

AVR cores running at 8–16 MHz are too slow for the standard (on-the-fly) variant, which requires
the host CPU to be approximately 48 MHz to feed the SPI peripheral between bytes.
The `prerendered` module solves this by pre-computing the entire SPI bit-stream into a
caller-provided buffer before transmission, then DMA-handing the buffer to SPI hardware.

Buffer sizing formula:

```
min_buffer_bytes = 12 * num_leds + 20
// or +40 bytes if using the mosi_idle_high feature
```

This is derived from the SPI encoding: each WS2812 data bit is encoded as 3 SPI bits, so
each 24-bit LED colour expands to 72 SPI bits = 9 bytes.
With the 4-SPI-bits-per-WS2812-bit encoding variant the crate uses, it is 12 bytes per LED.
The trailing 20 bytes represent the WS2812 reset pulse (≥ 50 µs low).

On a 16 MHz ATmega328P the available hardware SPI divisors (2, 4, 8, 16, ...) yield:
8 MHz (÷2), 4 MHz (÷4), 2 MHz (÷8).
The WS2812 protocol requires an effective SPI clock of 2–3.8 MHz, so **÷8 = 2 MHz** is the
only standard divisor that falls in the acceptable range.

**Maintenance**

- Repository: `smart-leds-rs/ws2812-spi-rs`
- Version: 0.5.1 (June 3, 2025)
- Stars: 98 | Forks: 31
- Open issues: not blocking
- Actively maintained; 83 commits total

**Fit with our philosophy**

Does not separate pure logic from hardware code.
No tests runnable on host.
Uses `embedded-hal 0.2`, which is now superseded by 1.0.
The prerendered mode's buffer-sizing formula and SPI-encoding approach are worth adopting.

</details>

<details>
<summary><strong>ws2812-avr (devcexx, September 2024)</strong></summary>

**Purpose**

A pure-Rust clockless bitbang WS2812 driver for AVR devices, created as a learning exercise.
Available only from GitHub (not published to crates.io).

**Architecture**

Uses Rust's unstable `generic_const_exprs` feature (`#![feature(generic_const_exprs)]`) to
compute timing loops at compile time rather than using inline assembly.
Targets chips by feature flag (e.g., `features = ["atmega328p"]`).
Requires Rust `nightly-2024-04-15` as a minimum.

**Known issues**

The `global_asm!` bug described above ([`rust-lang/rust#134758`](https://github.com/rust-lang/rust/issues/134758)) can affect this crate when
built as a library, since generic const expressions may interact with the feature propagation
issue.
The author explicitly cautions users that the library depends on unstable compiler features
and may break with nightly upgrades.
GPL-3.0 licence is **incompatible** with the `rustyfarian-ws2812` workspace (MIT/Apache-2.0).

**Maintenance**

- Repository: `devcexx/ws2812-avr`
- 9 commits, 0 stars (as of research date)
- Last activity: September 2024

**Fit with our philosophy**

The idea of computing timing windows from chip frequency at compile time via const generics is
clever and aligns with the pure-logic-in-types approach.
However, GPL licence, low maturity, zero crates.io presence, dependence on an unstable nightly
feature (`generic_const_exprs`), and the `global_asm!` interaction risk make this a research
reference rather than a dependency candidate.

</details>

<details>
<summary><strong>avr-hal / arduino-hal (Rahix, actively maintained)</strong></summary>

**Purpose**

The canonical Rust AVR hardware abstraction layer.
Provides `atmega-hal`, `attiny-hal`, and the higher-level `arduino-hal` for common Arduino boards.
Includes `avr-device` (SVD-generated register access), `avr-hal-generic` (macro-based HAL primitives),
and `ravedude` (flashing tool wrapping `avrdude`).

**embedded-hal 1.0 migration**

[Issue #77](https://github.com/Rahix/avr-hal/issues/77) ("Switch to embedded-hal 1.0.0") was **closed as completed on 4 January 2025**.
The migration was spread across multiple PRs; [PR #488](https://github.com/Rahix/avr-hal/pull/488) did the bulk of the work, and [PR #621](https://github.com/Rahix/avr-hal/pull/621)
(`[arduino-hal] remove embedded-hal-v0 dependency`) removed the last 0.2 traces.
This means `avr-hal` now implements `embedded-hal 1.0` `SpiBus` (replacing the old `FullDuplex`),
`OutputPin`, `InputPin`, and related traits.

**SPI on ATmega328P**

`arduino-hal` exposes `arduino_hal::Spi` which implements `embedded_hal::spi::SpiBus`.
On a 16 MHz ATmega328P, the hardware SPI divisor must be set to 8 (yielding 2 MHz)
to fall within the WS2812 acceptable range of 2–3.8 MHz.

**Nightly toolchain**

The current `rust-toolchain.toml` pins to `nightly-2025-04-27`.
The toolchain file is included in the template and is auto-applied by `rustup`.

**Maintenance**

- Repository: `Rahix/avr-hal`
- Stars: 1400+
- 663+ commits, 100+ contributors
- Latest tagged release present; actively maintained

</details>

---

## C / Arduino Landscape

Understanding existing C implementations is essential for knowing what a Rust driver must do.

<details>
<summary><strong>Adafruit NeoPixel library</strong></summary>

The Adafruit NeoPixel library is the most widely deployed WS2812 driver for Arduino.
The timing-critical inner loop is written in **AVR assembly language** with detailed C-style
comments, because the constraints (T0H ≤ 500 ns = ≤ 8 cycles at 16 MHz) cannot be reliably
met by compiled C.
The library stores one pixel value per RGB byte triplet in a heap-allocated buffer (3 bytes per LED),
limiting Arduino Uno to approximately 500 LEDs.
Non-destructive brightness scaling is described as not yet implemented for AVR due to the
assembly constraints, so brightness is applied destructively by modifying the stored buffer.
Interrupts must be disabled for the duration of a frame write.

</details>

<details>
<summary><strong>FastLED</strong></summary>

FastLED supports WS2812B natively alongside a wide range of other LED chipsets.
It uses assembly inner loops for AVR and hardware-specific DMA or timer paths for 32-bit platforms.
It exposes higher-level animation helpers (HSV, colour palettes, noise functions) above the
hardware layer — conceptually similar to how `ferriswheel` sits above `rustyfarian-esp-*`.
On ESP32-P4, FastLED repurposes the RGB LCD peripheral for zero-CPU-overhead WS2812 output,
illustrating the same platform-specific hardware mapping strategy used in this workspace.

</details>

<details>
<summary><strong>light_ws2812 (cpldcpu, v2.6, May 2024)</strong></summary>

The lightest-weight option: under 50 bytes of flash in most configurations.
Uses **cycle-optimised assembler inner loops** whose timing is auto-adjusted at compile time
via the `F_CPU` preprocessor macro.
Supports 8, 9.6, 12, 16, and 20 MHz clock speeds; 4 MHz support was removed in v2.5.
Disables interrupts by saving SREG and calling `cli()` before each frame write, restoring on exit.
Does not use any hardware peripheral — purely GPIO bitbang.
Last release: v2.6, May 1 2024; 232 commits; actively maintained.

The key technique is that **all timing is derived from `F_CPU` at compile time** rather than
measured or tuned at runtime.
A Rust equivalent would use a `const` CPU frequency parameter to generate compile-time pulse
widths, mirroring what `ws2812-avr` attempts with `generic_const_exprs`.

</details>

---

## Timing Approaches Compared

| Approach | How it works | Interrupt-free window | AVR suitability | Rust path |
|:---------|:-------------|:----------------------|:----------------|:----------|
| Bitbang + assembly | GPIO toggled by cycle-counted `nop` sequences | Required for entire frame | Works at 8–16 MHz but assembly is platform-specific | Inline `asm!` in Rust; blocked by `global_asm!` library bug |
| Bitbang + const generics | Compile-time loop counts derived from CPU freq | Required for entire frame | Proof-of-concept exists (`ws2812-avr`); fragile nightly dependency | `generic_const_exprs` (unstable) |
| SPI prerendered | Pre-encode all bits into a `[u8; N]` buffer; SPI hardware clocks it out | Required for entire frame (SPI TX); buffer can be prepared with interrupts enabled | Recommended for Rust on AVR; 2 MHz with ÷8 divisor on 16 MHz ATmega | `ws2812-spi` prerendered; `embedded-hal 1.0` `SpiBus::write` |
| SPI on-the-fly | Generate each byte just before SPI sends it | Required; CPU must keep pace | Requires ≥ 48 MHz; fails on AVR | Not viable for AVR |
| UART | Encode as UART frames at specific baud | Required | Possible but baud rates don't cleanly match WS2812 at standard AVR clocks | `ws2812-uart` crate exists; not AVR-validated |

**Conclusion**: the SPI prerendered approach is the correct Rust path for AVR.
It avoids inline assembly entirely, separates buffer preparation (pure logic, testable) from
transmission (hardware, thin wrapper), and maps directly onto the `embedded-hal 1.0` `SpiBus::write` method.

---

## RAM and Flash Constraints

The ATmega328P has **2 KB of SRAM** and **32 KB of flash**.
A 12-LED ring (the primary target of this workspace) requires a prerendered SPI buffer of
`12 × 12 + 20 = 164 bytes` — approximately 8% of available SRAM.
A 60-LED strip requires `12 × 60 + 20 = 740 bytes` — about 36% of SRAM before accounting for
the application stack and other data.
A 300-LED strip would require `3,620 bytes` which **exceeds available SRAM** and is not feasible.

AVR WS2812 use cases are therefore limited to short strips or single rings.
For the primary `ferriswheel` 12-LED use case, SRAM budget is comfortable.

---

## Challenges and Risks

<details>
<summary><strong>Permanent nightly dependency</strong></summary>

There is no timeline for AVR support reaching a stable Rust compiler.
Any AVR crate in the workspace would require a pinned nightly, which conflicts with the
rest of the workspace (esp-hal uses nightly for different unstable features; esp-idf uses
`cargo +esp` with a separate toolchain).
This means `rustyfarian-avr-ws2812` would need its own `rust-toolchain.toml` override or
a workspace-level toolchain management strategy that supports multiple nightly pins.

</details>

<details>
<summary><strong>embedded-hal version mismatch</strong></summary>

`avr-hal` now implements `embedded-hal 1.0` (since January 2025).
`ws2812-spi` v0.5.1 (June 2025) still targets `embedded-hal 0.2` via the `FullDuplex` / `blocking::spi::Write` traits.
A `rustyfarian-avr-ws2812` crate that targets `embedded-hal 1.0`'s `SpiBus` directly would need
to implement the SPI encoding itself rather than delegating to `ws2812-spi`.
This is a moderate amount of work but would produce a cleaner, future-proof crate.

</details>

<details>
<summary><strong>Interrupt-free window required</strong></summary>

WS2812 requires the data line to be held in a specific state without interruption for up to
`num_leds × 30 µs`.
For a 12-LED ring this is ≈ 360 µs; for 60 LEDs it is ≈ 1800 µs.
On AVR, this window must be achieved by disabling interrupts (`cli`) before the SPI transmission
begins and re-enabling them (`sei`) after.
In Rust the canonical approach is `avr_device::interrupt::free(|_| { ... })`.
This is safe to use but means LED updates create periodic latency spikes that will affect
anything relying on timer interrupts (e.g., the Arduino `millis()` timer tick).

</details>

<details>
<summary><strong>GNU toolchain installation burden</strong></summary>

Unlike the ESP toolchain (which `espup` installs), the AVR GNU toolchain must be installed via
the system package manager (`brew install avr-gcc` on macOS, `apt install gcc-avr` on Debian).
The CI pipeline would need to provision these tools, adding build complexity and OS-specific steps.
macOS Homebrew's `avr-gcc` package version may lag behind what LLVM requires (AVR-GCC 14.2.0
is the recommended version as of 2025; Homebrew may ship an older version).

</details>

<details>
<summary><strong>global_asm! library bug</strong></summary>

[`rust-lang/rust#134758`](https://github.com/rust-lang/rust/issues/134758) (open as of January 2025) prevents `global_asm!` in library crates
targeting AVR from correctly inheriting the target CPU feature set.
The workaround (setting `lto = false`) is straightforward for bitbang-based drivers.
The SPI prerendered approach does not use `global_asm!` and is therefore unaffected.

</details>

<details>
<summary><strong>No hardware in current workspace or CI</strong></summary>

The existing workspace CI targets host (aarch64/x86) and ESP32 (IDF + HAL).
Adding AVR would require either a third hardware target in CI or a pure compile-check job.
AVR compile checks require the GNU toolchain and a nightly Rust; this adds non-trivial CI surface.

</details>

---

## Comparison Table

| Item | `ws2812-spi` prerendered | `ws2812-avr` bitbang | New `rustyfarian-avr-ws2812` |
|:-----|:------------------------|:---------------------|:-----------------------------|
| Approach | SPI + pre-encoded buffer | Bitbang const generics | SPI + pre-encoded buffer |
| embedded-hal version | 0.2.4 | None (avr-hal direct) | 1.0 (target) |
| AVR support | Explicit, documented | ATmega328P (via features) | ATmega328P primary target |
| Nightly required | No (for host tests); Yes (for AVR target) | Yes (`generic_const_exprs`) | Yes (AVR target only) |
| Inline assembly | No | No | No |
| Pure/testable logic | No | No | Yes (buffer encoding in `ws2812-pure`) |
| Host-runnable tests | No | No | Yes (buffer encoding layer) |
| smart-leds-trait | 0.2.x | No | 0.3.x (embedded-hal 1.0) |
| Maintenance status | Active (v0.5.1, June 2025) | Low (9 commits, Sep 2024) | New |
| Licence | MIT OR Apache-2.0 | GPL-3.0 | MIT OR Apache-2.0 |
| Published on crates.io | Yes | No | Planned |
| SRAM usage (12 LEDs) | 164 bytes | ~0 (no buffer) | 164 bytes |

---

## Feasibility Assessment

**Is this viable?**

Yes, technically.
A `rustyfarian-avr-ws2812` crate is a realistic project with a clear implementation path.
The SPI prerendered approach eliminates the need for inline assembly, the buffer encoding
logic can live in `ws2812-pure` (keeping it host-testable), and the hardware wrapper reduces
to a small `SpiBus::write` call inside an interrupt-critical section.

**What approach makes sense?**

The crate should mirror the existing dual-HAL pattern:

1. **Extend `ws2812-pure`** with an `avr_prerender` function that takes a `&[RGB8]` slice and
   a `&mut [u8]` output buffer and fills it with the 3-SPI-bits-per-WS2812-bit encoding.
   This function is pure, `no_std`, and fully unit-testable on the host.

2. **`rustyfarian-avr-ws2812`** becomes a thin hardware wrapper: it holds the SPI bus and
   the static output buffer, calls `avr_prerender` from `ws2812-pure`, wraps the transmission
   in `avr_device::interrupt::free`, and calls `SpiBus::write`.
   The SPI frequency (2 MHz at ÷8 on a 16 MHz ATmega328P) must be configured by the caller or
   validated at construction time.

3. The crate targets `avr-unknown-gnu-atmega328` and optionally other chips via feature flags.
   A `rust-toolchain.toml` inside the crate directory pins the AVR nightly version independently
   of the rest of the workspace.

**What are the go/no-go conditions?**

Go conditions:
- Confirm `avr-hal`'s `SpiBus` implementation supports bulk `write` (not just single-byte `FullDuplex`) — the January 2025 embedded-hal 1.0 migration should have delivered this.
- Confirm `ws2812-spi` or a replacement targeting `embedded-hal 1.0` is available, or accept implementing the SPI encoding in `ws2812-pure`.
- Accept the permanent nightly toolchain dependency for the AVR crate.

No-go conditions:
- If CI cannot be extended to provision `avr-gcc` 14.x — compile-check coverage would be lost.
- If the workspace requires a single `rust-toolchain.toml` for all crates — AVR nightly and ESP nightly are currently different pins.

---

## Strategic Recommendation

**Adopt the SPI prerendered approach, but do not commit to the full crate yet.**

The highest-value action is an intermediate step: **extend `ws2812-pure`** with a
`prerender_spi` function that encodes an `RGB8` slice into a WS2812 SPI byte buffer.
This is pure logic, zero hardware dependencies, fully testable on the host, and immediately
useful for both a future `rustyfarian-avr-ws2812` crate and for verifying the AVR buffer
encoding is correct before any hardware is involved.
This aligns with the project philosophy and adds test coverage with no toolchain complexity.

For the hardware wrapper itself:

- **Monitor**: watch whether `ws2812-spi` publishes a version targeting `embedded-hal 1.0`
  `SpiBus` (replacing the current `embedded-hal 0.2` dependency) before writing a new crate.
  If it does, `rustyfarian-avr-ws2812` can be a thin 50-line wrapper around it.

- **If building from scratch**: the encoding belongs in `ws2812-pure`, the wrapper should hold
  only SPI bus + buffer, use `avr_device::interrupt::free` for interrupt safety, and target the
  `avr-unknown-gnu-atmega328` target with a per-crate `rust-toolchain.toml` override.

- **Do not adopt `ws2812-avr`** (devcexx): GPL licence, zero crates.io presence, dependence on
  `generic_const_exprs` (deeply unstable), and a nine-commit history make it unsuitable as a
  dependency or fork base.

- **Do not attempt bitbang in Rust without assembly**: pure-Rust bitbang without `asm!` cannot
  reliably hit the T0H ≤ 500 ns constraint in a library context, especially in debug builds.
  The SPI prerendered approach sidesteps this entirely.

---

*Research date: 2026-03-12*

Sources:
- [The AVR-Rust Guidebook: avr-unknown-gnu-atmega328 target](https://book.avr-rust.org/003.1-the-avr-unknown-gnu-atmega328-target.html)
- [Rahix/avr-hal on GitHub](https://github.com/Rahix/avr-hal)
- [avr-hal embedded-hal 1.0 migration issue #77](https://github.com/Rahix/avr-hal/issues/77)
- [avr-hal nightly-2025-01-01 upgrade PR #585](https://github.com/Rahix/avr-hal/pull/585)
- [rust-lang/rust#134758 — global_asm! AVR library bug](https://github.com/rust-lang/rust/issues/134758)
- [smart-leds-rs/ws2812-spi-rs on GitHub](https://github.com/smart-leds-rs/ws2812-spi-rs)
- [ws2812-spi 0.4.0 prerendered API on docs.rs](https://docs.rs/ws2812-spi/0.4.0/ws2812_spi/prerendered/struct.Ws2812.html)
- [ws2812-spi 0.5.1 on crates.io](https://crates.io/crates/ws2812-spi)
- [devcexx/ws2812-avr on GitHub](https://github.com/devcexx/ws2812-avr)
- [cpldcpu/light_ws2812 on GitHub](https://github.com/cpldcpu/light_ws2812)
- [adafruit/Adafruit_NeoPixel on GitHub](https://github.com/adafruit/Adafruit_NeoPixel)
- [Adafruit NeoPixel Uberguide — Advanced Coding](https://learn.adafruit.com/adafruit-neopixel-uberguide/advanced-coding)
- [FastLED library](https://fastled.io/)
- [WS2812 vs. Rust — sawatzke.dev](https://sawatzke.dev/blog1/ws2812-rust/)
- [smart-leds-trait on docs.rs](https://docs.rs/smart-leds-trait/latest/smart_leds_trait/)
- [NeoPixels timing analysis — josh.com](https://wp.josh.com/2014/05/13/ws2812-neopixels-are-not-so-finicky-once-you-get-to-know-them/)
- [AVR SPI NeoPixel approach — LibStock](https://libstock.mikroe.com/projects/view/1953/driving-adafruit-s-neopixel-ws2812-strip-with-avr-s-hardware-spi)
- [Bit-banging WS2812 in Rust — Hackster.io](https://www.hackster.io/dcaponi1/bit-banging-ws2812-in-rust-bb30bc)
- [QMK WS2812 driver docs](https://docs.qmk.fm/drivers/ws2812)
- [How to program an Arduino using Rust (2024) — rybicki.io](https://rybicki.io/blog/2024/04/16/program-arduino-using-rust.html)
