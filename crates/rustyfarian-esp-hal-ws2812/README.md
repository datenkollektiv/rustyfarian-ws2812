# rustyfarian-esp-hal-ws2812

[![crates.io version](https://img.shields.io/crates/v/rustyfarian-esp-hal-ws2812.svg)](https://crates.io/crates/rustyfarian-esp-hal-ws2812)
[![docs.rs docs](https://img.shields.io/docsrs/rustyfarian-esp-hal-ws2812)](https://docs.rs/rustyfarian-esp-hal-ws2812)

Bare-metal `no_std` WS2812 (NeoPixel) LED driver using the `esp-hal` RMT peripheral.

Drives WS2812/NeoPixel addressable LEDs on ESP32-C3, ESP32-C6, ESP32-S3, and
classic ESP32 targets without an OS or ESP-IDF. The driver is const-generic over the
pulse-code buffer size, allocates nothing at runtime, and supports both
blocking and `async` (Embassy) operation. Implements `SmartLedsWrite` for
the `smart-leds` ecosystem. For std/ESP-IDF projects see
`rustyfarian-esp-idf-ws2812`.

Part of the [`rustyfarian-ws2812`](https://github.com/datenkollektiv/rustyfarian-ws2812)
workspace alongside [`rustyfarian-esp-idf-ws2812`](https://crates.io/crates/rustyfarian-esp-idf-ws2812)
(std, ESP-IDF), [`rustyfarian-avr-ws2812`](https://crates.io/crates/rustyfarian-avr-ws2812)
(AVR), and the pure-logic crates [`ferriswheel`](https://crates.io/crates/ferriswheel),
[`pennant`](https://crates.io/crates/pennant), and [`bunting`](https://crates.io/crates/bunting).

## Example

The default feature enables `esp32c6`. To target a different chip, disable
defaults and select exactly one chip feature.

**ESP32-C6** (default — no `Cargo.toml` override needed):

```toml
rustyfarian-esp-hal-ws2812 = { version = "0.9" }
```

**ESP32-C3:**

```toml
rustyfarian-esp-hal-ws2812 = { version = "0.9", default-features = false, features = ["esp32c3", "unstable", "pennant"] }
```

**Classic ESP32 / WROOM-32** (requires the `esp` Xtensa toolchain):

```toml
rustyfarian-esp-hal-ws2812 = { version = "0.9", default-features = false, features = ["esp32", "unstable", "pennant"] }
```

**ESP32-S3** (requires the `esp` Xtensa toolchain):

```toml
rustyfarian-esp-hal-ws2812 = { version = "0.9", default-features = false, features = ["esp32s3", "unstable", "pennant"] }
```

```rust
use esp_hal::{rmt::{Rmt, TxChannelCreator}, time::Rate};
use rgb::RGB8;
use rustyfarian_esp_hal_ws2812::{Ws2812Rmt, buffer_size, tx_channel_config};

const NUM_LEDS: usize = 12;
const N: usize = buffer_size(NUM_LEDS);

let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
let channel = rmt.channel0.configure_tx(&tx_channel_config()).unwrap().with_pin(peripherals.GPIO18);

let mut led: Ws2812Rmt<_, N> = Ws2812Rmt::new(channel);
led.set_pixel(RGB8::new(255, 0, 0)).unwrap();
```

`Ws2812Rmt::from_channel_creator(rmt.channel0, peripherals.GPIO18)` does the same configuration and wiring in one call, and returns `Result<Self, Error>`.
Both assume `Rmt::new(peripherals.RMT, Rate::from_mhz(80))`.
For another source rate, use `tx_channel_config_for(source)`, which returns `Result<TxChannelConfig, Error>` and `Error::RmtConfig` when no exact divider exists.

## Long Frames

A frame that fits in one RMT channel RAM block is sent the same way in blocking and async mode.
That block is 48 codes on ESP32-C3, ESP32-C6 and ESP32-S3, and 64 codes on ESP32.
Longer frames, which is two or more LEDs on the first three and three or more on ESP32, follow the `LongFramePolicy` set with `with_long_frame_policy`.
The policy applies to both `Ws2812Rmt<'d, Blocking, N>` and `Ws2812Rmt<'d, Async, N>`.

| Policy                      | Long frame behaviour                                                  | Interrupts                        | Use when                                          |
|:----------------------------|:----------------------------------------------------------------------|:----------------------------------|:--------------------------------------------------|
| `CriticalSection` (default) | Refill runs with interrupts masked; capped by the masked-time ceiling | Masked, about 1.1–1.3 µs per code | Radio firmware that accepts a short masked window |
| `Poll`                      | Refill busy-polls with interrupts live, bounded by the same timeout   | Live                              | Long strips on firmware without a radio           |
| `Reject`                    | Returns `Error::FrameTooLong` and sends nothing                       | Never masked                      | Firmware that must never mask or corrupt output   |

`CriticalSection` costs about 0.36 ms for 12 LEDs and 1.9 ms for 64 LEDs with interrupts masked.
The ceiling caps the estimated nominal masked time, 1.3 µs per code, at a default of 1000 µs (32 LEDs); a longer frame returns `Error::FrameTooLong` and sends nothing.
The ceiling limits the estimate; it is not a hard bound on interrupts-off time.
For a longer strip, lift the cap with `with_max_nominal_masked_us(None)` or set a higher value, or choose `Poll` or `Reject`.
`FrameTooLong` repeats on every frame that exceeds the ceiling, so log it once at start-up rather than per frame.
`Poll` is not radio-safe: radio and other interrupts can delay the refill, so use it only where that is acceptable.
On a stalled refill, the actual masked window is bounded by the timeout: 2 µs per code plus 1 ms, about 2.5 ms at 769 codes.
A blocking call is bounded to about two timeout budgets: the timeout, then an unmasked drain for a second budget.
If the drain ends the frame, `Error::Transmit` is returned and the driver keeps its channel; the frame failed, and the driver is usable.
If the drain does not end the frame, the transaction is dropped and the driver loses its channel.
That call returns `Error::ChannelLost`, and every later call returns `Error::ChannelLost` until the driver is rebuilt.
`Error::Transmit` means this frame failed, possibly garbled, whether from a timeout, a stopped clock or a hardware error; the driver is still usable.
`Error::ChannelLost` means the driver must be rebuilt.
Do not retry a failed `Poll` frame under radio load, because it keeps failing.
Rebuilding needs `Rmt` created from `peripherals.RMT.reborrow()` and the pin passed via `reborrow()`.
Otherwise only a reset recovers the driver.
Rebuilding is not hardware-tested.
A dead classic ESP32 peripheral can still hang in the drop, which fills the end marker and waits for End.
On an async timeout, the future is dropped and the frame is lost; the async driver does not lose its channel.
Frames that fit in channel RAM are not bounded by the timeout or the ceiling.

The deprecated `set_pixels_slice_masked` always masks interrupts, regardless of length and policy; the default policy covers that use case, so call `set_pixels_slice` instead.
Firmware with a radio that uses the async `CriticalSection` path should build with `ESP_HAL_CONFIG_PLACE_RMT_DRIVER_IN_RAM=true`.
The only evidence that requires it is an earlier async bench, which garbled 12-LED frames with Wi-Fi active until the setting was enabled.
For the blocking driver, the setting is recommended as margin only.
The blocking masked path was clean on a downstream ESP32-C3 bench with Wi-Fi active in three runs at `f53c5e8`, without the setting.
The crate cannot detect this setting, so the choice is yours to make.
A downstream ESP32-C3 bench with Wi-Fi active saw `Poll` garble frames.
No option is proven to keep long frames correct under radio load on esp-hal 1.2.2.

Every frame waits at least 280 µs after the previous one so the LEDs latch.
Async frames that fit in channel RAM carry this interval in the signal, so they cost no CPU time but complete about 280 µs later.
Other frames busy-wait the remainder before the next frame, and this needs esp-hal's `rt` feature.

Hardware validation so far: one ESP32-C3 with 12 LEDs and Wi-Fi active, on a downstream bench.
Not validated: other chips, longer strips, the async path, and timeout or drop recovery.
The hardware waveform and the radio margin are not yet validated.
Test Wi-Fi, ESP-NOW and UART traffic with your real strip length.
See the crate documentation for the full caveats.

## Errors

`Error` implements `core::error::Error`, so it works with error-reporting code in `no_std`.
`Error::Transmit` means the frame failed and the driver is usable.
`Error::ChannelLost` means the driver has no RMT channel, and it stays that way until the driver is rebuilt.
`Error` is `#[non_exhaustive]`, so match with a wildcard arm.

## Supported Targets

| Feature             | Target triple                  | Boards                                                  |
|:--------------------|:-------------------------------|:--------------------------------------------------------|
| `esp32c6` (default) | `riscv32imac-unknown-none-elf` | ESP32-C6-DevKitC-1                                      |
| `esp32c3`           | `riscv32imc-unknown-none-elf`  | ESP32-C3-DevKit-Rust-1, ESP32-C3-DevKitC-02             |
| `esp32`             | `xtensa-esp32-none-elf`        | ESP32-WROOM-32 (requires `esp` toolchain)               |
| `esp32s3`           | `xtensa-esp32s3-none-elf`      | Elecrow CrowPanel 1.28" knob (requires `esp` toolchain) |

## Features

| Feature       | Default | Description                                                                                   |
|:--------------|:--------|:----------------------------------------------------------------------------------------------|
| `esp32c6`     | yes     | chip support for ESP32-C6                                                                     |
| `esp32c3`     | no      | chip support for ESP32-C3                                                                     |
| `esp32`       | no      | chip support for classic ESP32 / WROOM-32                                                     |
| `esp32s3`     | no      | chip support for ESP32-S3                                                                     |
| `async`       | no      | async `Ws2812Rmt<'d, Async, N>` impls; no executor or timer                                   |
| `embassy`     | no      | Embassy executor, timer and sync runtime; implies `async`; required by the `*_async` examples |
| `pennant`     | yes     | implements `StatusLed`; `AsyncStatusLed` when `async` is also enabled                         |
| `smart-leds`  | no      | enables `smart-leds` brightness and gamma adapters                                            |
| `rt`          | no      | enables `esp-hal` runtime (for `#[entry]` examples)                                           |
| `esp-println` | no      | enables `esp-println` for debug output in examples                                            |

## Documentation

Full API docs at [docs.rs/rustyfarian-esp-hal-ws2812](https://docs.rs/rustyfarian-esp-hal-ws2812).

## License

Dual-licensed under MIT or Apache-2.0.

## Changelog

See the [workspace CHANGELOG](https://github.com/datenkollektiv/rustyfarian-ws2812/blob/main/CHANGELOG.md)
for release notes across all crates.
