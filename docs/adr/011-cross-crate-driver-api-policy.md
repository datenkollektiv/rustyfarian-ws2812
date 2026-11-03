# ADR 011: Cross-Crate Driver API Policy

## Status

Accepted (2026-11-03)

## Context

[ADR 010](010-interchangeable-driver-semantics.md) aligned the runtime behaviour of the two ESP drivers.
It left out the third driver crate, the error types, the Cargo feature names, the shared WS2812 constants and the build tooling.
The [cross-platform streamlining feature](../features/cross-platform-streamlining-v1.md) collected eight open questions about those, and this ADR records the answers as policy, so the next driver crate does not reopen them.

Findings from the 2026-11-03 investigation that changed the premises:

1. **The ESP-IDF driver does not emit the 350 ns T0H it asks for.**
   esp-idf-hal 0.47 `duration_to_ticks` is an integer division (`src/rmt/pulse.rs:185`), so 350 ns at the 10 MHz resolution becomes 3 ticks, 300 ns.
   The esp-hal driver uses 4 ticks, 400 ns, with a comment quoting the original WS2812 tolerance.
2. **ESP-IDF `new()` cannot succeed on a classic ESP32 or S2.**
   ESP-IDF v5.3.3 rejects a `mem_block_symbols` below the chip's block size (`components/esp_driver_rmt/src/rmt_tx.c:247`), and `new()` hardcodes 48, while those two chips have 64-word blocks.
   The feature doc assumed `idf_esp32_rainbow` runs with 48 symbols; the example was never flashed.
3. **Datasheet windows differ by revision.**
   Datasheet research on 2026-11-03 (not re-measured here) read the Worldsemi WS2812B-V5 sheet (V6.1) as T0H 220 to 380 ns and reset at least 280 µs, while the original WS2812 and the older WS2812B sheets allow 350 or 400 ns ±150 ns.
   300 ns is inside every window found; 400 ns is above the V5 maximum.
4. **The AVR crate speaks a different dialect.**
   Its only send method is `write(&[RGB8])`, it has no `pennant::StatusLed` impl, and its `SmartLedsWrite` impl sits behind a feature that CI compiles only on the host.
5. **Three feature names for two dependencies.**
   `ferriswheel` calls its `smart-leds-trait` dependency `smart-leds-compat`; the AVR crate calls it `smart-leds-trait`; the ESP crates use `smart-leds` for the `smart-leds` helper crate.
6. **The downstream picture.**
   One downstream project wraps the ESP-IDF driver's errors with anyhow's `.context()`, and two call `pennant::PulseEffect::update` with `(u8, u8, u8)` tuples in six places.

## Decision

**Every driver crate exposes the same vocabulary, a typed error, the same optional integrations and the same WS2812 constants, and a difference between crates is stated with its reason.**

### One vocabulary

- Every driver has `set_pixel(RGB8)`, which sends a one-LED frame and addresses only the first LED, and `set_pixels_slice(&[RGB8])`.
- The AVR drivers rename `write` to `set_pixels_slice` and gain `set_pixel`.
  `write` stays as a `#[deprecated(since = "0.8.0")]` alias for the 0.8 cycle.
- A method with the same name means the same thing on every driver, as [ADR 010](010-interchangeable-driver-semantics.md) already requires for the ESP pair.

### Typed errors everywhere

- Every driver returns its own error type, which implements `core::error::Error`, `Debug` and `Display`.
  No library API returns `anyhow::Error`.
- The ESP-IDF driver gains `Error`, `#[non_exhaustive]`, `Copy` and `Eq`, with variants `ResolutionTooLow { hz }`, `Channel(EspError)`, `Timing(EspError)`, `Encoder(EspError)` and `Transmit(EspError)`.
  `source()` returns the inner `EspError`.
  The `CONFIG_RMT_ISR_IRAM_SAFE` hint moves into the `Display` of `Channel` when the cfg is set and the code is `ESP_ERR_INVALID_ARG`.
- `anyhow` becomes a target-gated dev-dependency of the ESP-IDF crate, used by its examples only.
- Consumers lose anyhow's context chain inside the driver and gain `match`, `Copy` and no allocation per error.
  `?` into `anyhow` and `.context()` on the result keep working because the type is `Send + Sync + 'static`.

### Optional integrations

- `pennant::StatusLed` is implemented by every driver behind a feature named `pennant`.
  The ESP crates enable it by default; the AVR crate, which has no default features, leaves it off.
  `set_color` delegates to `set_pixel`.
- `SmartLedsWrite` is always implemented on the ESP crates and stays opt-in on the AVR crate behind `smart-leds-trait`.
  The reason is the AVR adapter's 768-byte stack buffer, 37 % of the ATmega328P's SRAM, which should remain a conscious choice.
  The feature joins the avr-none check recipes so that path is compiled for the real target in CI.
- `pennant::PulseEffect::update` takes `impl Into<RGB8>`, which accepts `RGB8` and the `(u8, u8, u8)` tuples downstream code passes today.

### Feature names

A Cargo feature that enables an optional dependency is named after that dependency.

- AVR `smart-leds-trait` and ESP `smart-leds` already comply.
- `ferriswheel` gains `smart-leds-trait`; `smart-leds-compat` stays as an alias of it for the 0.8 cycle and is removed in a later minor.

### Shared WS2812 constants

- `bunting` owns the WS2812 pulse timings in nanoseconds, `WS2812_T0H_NS = 300`, `WS2812_T0L_NS = 800`, `WS2812_T1H_NS = 700`, `WS2812_T1L_NS = 600`, and `WS2812_RESET_US = 280`.
- `bunting` provides a `const fn` that converts nanoseconds to ticks at a given tick period, and a compile-time check that every pulse timing is a whole number of 100 ns ticks, so a value that silently rounds cannot return.
- The esp-hal driver derives its tick constants from those values, which changes its T0H from 4 ticks to 3.
  The ESP-IDF driver passes the nanosecond values to `Pulse::new_with_duration`; its waveform does not change.
- Both ESP drivers re-export `WS2812_RESET_US` from `bunting`, so existing imports keep compiling.

### ESP-IDF block size

`Ws2812Rmt::new()` uses `esp_idf_hal::sys::SOC_RMT_MEM_WORDS_PER_CHANNEL` instead of 48.
That is 48 on ESP32-C3, C6, S3 and H2 and 64 on ESP32 and S2.
`new_with_channel_config` stays for other tuning.

### Build tooling

`just build-example <example>` takes only the example name and infers the crate from the `hal_` or `idf_` prefix, as `just flash` and `just run` already do.
The `build-example-*` aliases are removed; the example name is the shortcut.
This closes the roadmap item on alias coverage.

## Consequences

### Positive

- The same code drives a ring through any of the three driver crates with the same method names and the same error handling pattern.
- One place holds the WS2812 timings, and a compile-time check catches tick rounding.
- `Ws2812Rmt::new()` works on every ESP32 variant that ESP-IDF supports.
- Feature tables read the same across the crates.

### Negative

- **Breaking:** the ESP-IDF driver's error type changes for code that names `anyhow::Error` as its error.
- **Breaking, deprecations:** AVR `write` warns, and `ferriswheel`'s `smart-leds-compat` is an alias that will go.
- **Hardware:** the esp-hal T0H change invalidates every earlier esp-hal sign-off.
  The C3 and C6 rings and the S3 CrowPanel were re-flashed and confirmed on 2026-11-03 (see the feature doc for the ELF hashes).
  The classic ESP32 ran the ESP-IDF rainbow; its esp-hal pulses are listed in the feature doc.
- The AVR crate gains an optional path dependency on `pennant`, which adds a publish-order edge.
- The `SmartLedsWrite` asymmetry between ESP and AVR stays, with its reason written down.

### Residual risks

- 300 ns T0H on the esp-hal driver is confirmed on the C3, C6 and S3 (2026-11-03); the classic ESP32 has no run.
- The V5 timing window comes from a datasheet reading, not from a measurement on these rings.
- The block-size fix ran on both block sizes on 2026-11-03: 48 words on the C3 and S3, 64 words on a classic ESP32 rev v3.1 (`idf_esp32_rainbow`, its first run); the S2 has no board.

## Related

- [ADR 005: Dual-HAL Strategy](005-dual-hal-strategy.md)
- [ADR 007: AVR WS2812 Driver Strategy](007-avr-ws2812-driver-strategy.md)
- [ADR 010: Interchangeable Driver Semantics](010-interchangeable-driver-semantics.md)
- [Feature: Cross-platform API streamlining v1](../features/cross-platform-streamlining-v1.md)
