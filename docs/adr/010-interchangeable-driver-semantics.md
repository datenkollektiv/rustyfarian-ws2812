# ADR 010: Interchangeable Driver Semantics

## Status

Accepted (2026-11-01)

## Context

A downstream project, `rustyfarian-rgb-clock`, runs the same 12-LED clock on the esp-hal and ESP-IDF drivers.
The same calls behave differently on the two, and the request was filed on 2026-11-01.
Four gaps explain it:

1. **The long-frame policy is async-only on esp-hal.**
   [ADR 009](009-long-frame-policy-and-latch.md) added `LongFramePolicy` for `Ws2812Rmt<'d, Async, N>` only.
   Blocking `set_pixel`, `set_pixels_slice` and `SmartLedsWrite::write` poll the refill unmasked, so a frame longer than one channel RAM block garbles under Wi-Fi.
2. **The ESP-IDF driver enforces no reset interval.**
   Two frames sent back to back give the strip no reset interval, so the second frame may not be latched as a new frame.
3. **The ESP-IDF radio wording overstates the hardware.**
   The refill runs in the RMT interrupt service routine (ISR), which tolerates task latency but is not hard real time.
   That ISR is not IRAM-safe by default (`CONFIG_RMT_ISR_IRAM_SAFE` defaults to `n` in ESP-IDF v5.3.3, and this repository's `sdkconfig.defaults` does not set it), and it runs at a low or medium shared interrupt priority.
(The recommendation to enable the setting that followed from this point was wrong; see the correction section below.)
4. **Consumers copy the channel setup and cannot treat `Error` generically.**
   Every esp-hal consumer repeats the RMT channel configuration, and `Error` does not implement `core::error::Error`.

The unreleased 0.8 already carries breaking changes from [ADR 009](009-long-frame-policy-and-latch.md), so the semantic changes land in the same minor.

## Decision

Methods with the same name mean the same thing on both drivers.
Where the hardware constrains a path, the driver enforces it, and the documentation states what is and is not guaranteed.

### Long-frame policy applies to both modes on esp-hal

- Frames that fit one channel RAM block are unaffected in both modes.
  That block is 48 codes on ESP32-C3, ESP32-C6 and ESP32-S3, and 64 codes on ESP32.
  Two or more LEDs overflow the 48-code block, and three or more overflow the 64-code block.
- Longer frames follow `LongFramePolicy` on the blocking driver as well as the async driver.
  This changes [ADR 009](009-long-frame-policy-and-latch.md)'s statement that plain blocking writes are not a fallback: the default blocking path now masks long frames, within the timeout below.
- `set_pixel` (25 codes) always takes the direct path.

`LongFramePolicy` has three variants:

- `CriticalSection` (default) sends the refill with interrupts masked, bounded by a timeout.
- `Reject` returns `Error::FrameTooLong` and sends nothing.
- `Poll` busy-polls the refill with interrupts live, bounded by the same timeout.

### `Poll` is an explicit, non-radio-safe option

`Poll` is not radio-safe.
Radio and other interrupts can delay the refill, so it is an opt-in for long strips on firmware without a radio.

[ADR 009](009-long-frame-policy-and-latch.md) rejected `Await`, and `Poll` is acceptable for different reasons:

- `Await` depends on executor latency, because the refill runs in `TxFuture::poll`.
  Its deadline depends on when the executor schedules the task, so it bounds nothing.
- `Poll` is a tight loop.
  Only interrupts delay it, the timeout bounds it, and the docs state that it is not radio-safe.

### The blocking masked path is bounded

The blocking `CriticalSection` path polls `TxTransaction::poll()` (esp-hal 1.2.2, non-consuming) inside the critical section, with the timeout of 2 µs per code plus 1 ms.
The timeout bounds the polling window, with interrupts off, not the whole call.
On timeout, the driver leaves the critical section, and `wait()` finishes the frame with interrupts live.
That output is likely garbled, and the driver returns `Error::Transmit` and keeps the channel.
On the async `CriticalSection` path, the future is dropped after the critical section on timeout, which stops the channel.
Frames that fit in channel RAM, including `set_pixel`, keep the plain transmit and `wait()` with no timeout.
A late poll cannot garble them, and a timeout would report false errors.
Only `Poll`, `CriticalSection` and `set_pixels_slice_masked` are bounded.
The poll loop (`start_and_poll`) is placed in RAM with `#[esp_hal::ram]`, like the async `poll_to_completion`.
On the async driver, `Poll` runs `poll_to_completion` without the critical section, so the executor is still blocked for the frame.
If a blocking transmit fails to start, esp-hal returns the channel with the error (`(Error, Self)`), so the driver keeps its channel; this fixes a loss that predates this change.
The 2026-11-01 amendment below changes what happens after a blocking timeout.

### `set_pixels_slice_masked` stays

`set_pixels_slice_masked` always masks interrupts, regardless of length and policy.
Its deprecation is deferred, because downstream code uses it.

### Error

- `Error` implements `core::error::Error`.
- The documentation of `Error::FrameTooLong` covers `Reject` on both modes, not only async.
- `Error` remains `#[non_exhaustive]`, as decided in [ADR 009](009-long-frame-policy-and-latch.md).

### Channel configuration

- `tx_channel_config() -> TxChannelConfig` returns the divider, idle-low output, idle output enabled and no carrier that the driver requires.
- `Ws2812Rmt::from_channel_creator(creator, pin) -> Result<Self, Error>` configures the channel and wraps it in one call.
  `ConfigError` maps to `Error::RmtConfig`.

Callers no longer copy the RMT setup.

### ESP-IDF latch

- `pub const WS2812_RESET_US: u32 = 280` is not configurable, as in [ADR 009](009-long-frame-policy-and-latch.md).
- The driver stores `next_frame_at` and waits for the remainder before the next frame with `Ets::delay_us`, which avoids FreeRTOS tick rounding.
- The driver stamps `next_frame_at` after `wait_all_done`, on success and on error.

### ESP-IDF radio wording

The ESP-IDF docs describe the refill as tolerant of task latency, not hard real time.
~~Firmware that writes flash (NVS or OTA) while animating should set `CONFIG_RMT_ISR_IRAM_SAFE=y`.~~
Withdrawn 2026-11-01: enabling the setting makes `Ws2812Rmt::new` fail with esp-idf-hal 0.47.0.
Firmware should not enable it; see the correction section below.
This repository does not change `sdkconfig.defaults`, because the setting belongs to each firmware.

### Not in scope

- A shared `Ws2812` trait across crates.
  `SmartLedsWrite` is the common seam.
- Removing the ESP-IDF driver's `Vec` allocation.

## Consequences

### Positive

- The same call has the same error and timing semantics on both drivers, where the hardware allows it.
- Blocking callers get a bounded masked path and `Poll` without moving to async.
- The ESP-IDF driver honours the WS2812 reset interval.
- `Error` works with generic `core::error::Error` consumers, and channel setup is one call.

### Negative

- Blocking `set_pixels_slice` and `SmartLedsWrite::write` now mask interrupts for long frames by default.
  That costs about 1.25 µs per code: about 0.36 ms for 12 LEDs and 1.9 ms for 64 LEDs.
- `Poll` is not radio-safe, so a caller who chooses it under radio load can still receive garbled frames.
  The docs state this, but the crate does not enforce it.
- The ESP-IDF driver busy-waits up to 280 µs between back-to-back frames.
- The change is breaking for blocking long-frame timing and for `Error` matches, and it ships in 0.8.

### Residual risks

- **Hardware validation is partial.**
  The downstream re-benches on a C3 with Wi-Fi active cover the blocking default policy, `SmartLedsWrite`, `set_pixels_slice_masked` and `Poll` (see [Hardware results and ChannelLost](#hardware-results-and-channellost-2026-11-01)).
  Earlier sign-offs of the unbounded-`wait()` binaries and the async `CriticalSection` path do not carry over.
  Still open: the async path, other chips, longer strips, the waveform capture, and the drop and rebuild path.
- **A dead ESP32 peripheral can still spin in `TxGuard::drop`.**
  Replaces the earlier "unbounded recovery `wait()`" risk; see the amendment below.
  After a blocking timeout that does not drain, the transaction is dropped.
  On classic ESP32, the drop fills the end marker and spins until End, so a peripheral that never reaches End still hangs there.
- **A stopped clock ends the poll loop after milliseconds.**
  The loop gives up after 1000 stalled clock reads, with 16 polls between reads, so the bound is milliseconds, not sub-millisecond.
  This applies only to an `unsafe` setup that skips `esp_hal::init`.
- **Radio contention** on shared resources remains the working hypothesis from [ADR 009](009-long-frame-policy-and-latch.md).
- **The ESP-IDF refill ISR** can be delayed during flash writes, and the setting that would prevent it fails with esp-idf-hal 0.47.0.
  One clean run of a 1.6 MB OTA download (about 17 s, about 170 frames, no analyser) is the only evidence.
  The upstream fix is tracked as a follow-up in the feature doc.
- **The esp-hal latch wait and critical-section implementation** need the `rt` feature, as in [ADR 009](009-long-frame-policy-and-latch.md).

## Amendment: bounded recovery and masked-time ceiling (2026-11-01)

A review of commit `2931479` found two gaps in the decisions above.
This amendment records the changes; the sections above are kept as first accepted.

### Blocking timeout drains, then drops

The blocking timeout no longer returns the channel after `wait()`.

- On timeout, the frame drains unmasked for a second budget.
- If the frame ends in that budget, the channel is kept and `Error::Transmit` is returned.
- If it does not end, the transaction is dropped.
  The drop guard stops the channel: immediately on ESP32-C3, C6 and S3, and on classic ESP32 by filling the end marker and spinning until End.
- The driver then has no channel.
  ~~Every later call returns `Error::Transmit`, so the lost-channel state shows as repeated `Error::Transmit`.~~
  Superseded by the `ChannelLost` section below: every later call returns `Error::ChannelLost`.
- Rebuilding the driver needs `Rmt` created from `peripherals.RMT.reborrow()` and the pin passed via `reborrow()`.
  Otherwise only a reset recovers the driver.
  Rebuilding is not hardware-tested.

The blocking call is therefore bounded to about two timeout budgets: the timeout, then the drain.

esp-hal 1.2.2 offers no stop-and-return-channel for single-shot transmits; only `ContinuousTxTransaction::stop` exists.
That is why the channel is lost on a failed drain.
The blocking call is bounded, except for a dead classic ESP32 peripheral, which can still spin in `TxGuard::drop`.

### Masked-time ceiling

- `DEFAULT_MAX_NOMINAL_MASKED_US = 1000` (named `DEFAULT_MAX_MASKED_US` until the 2026-11-01 PR review).
  A `CriticalSection` frame whose estimated nominal masked time (codes × 1.3 µs) exceeds the ceiling returns `Error::FrameTooLong` on both the blocking and the async driver, and sends nothing.
  The ceiling caps the nominal estimate only.
  On a stalled refill, the actual masked window is bounded by the timeout (2 µs per code plus 1 ms, about 2.5 ms at 769 codes).
- With the default, 32 LEDs (769 pulse codes) fit under the ceiling.
  A longer strip calls `with_max_nominal_masked_us(None)` to lift the cap, or sets a higher value, or uses `Poll`.
- `set_pixels_slice_masked` is not capped.
  It always masks, as before.
- The 64-LED sweep example lifts the ceiling.

### Consequences of the amendment

- **Negative:** a failed blocking drain loses the channel, so the caller must recreate the driver.
- **Negative:** `FrameTooLong` now also reports the masked-time ceiling, so code that matches on `FrameTooLong` sees a new cause.
- **Negative, breaking:** long `CriticalSection` strips that previously sent now fail with `FrameTooLong` until they lift or raise the ceiling.
- **Positive:** a blocking call is bounded to about two timeout budgets, except on a dead classic ESP32 peripheral, and a failed drain never returns a garbled frame as success.

## Hardware results and ChannelLost (2026-11-01)

A downstream project re-benched commit `f53c5e8` (clean tree) on 2026-11-01.
The setup was an ESP32-C3 (rev v0.3) with a 12-LED ring on GPIO4 and Wi-Fi active, including the SoftAP portal.
The firmware was built without `ESP_HAL_CONFIG_PLACE_RMT_DRIVER_IN_RAM`, and it used the blocking driver through `from_channel_creator`.

| Run | Call                                                                                            | Result                                                                                               |
|:----|:------------------------------------------------------------------------------------------------|:-----------------------------------------------------------------------------------------------------|
| R1  | `set_pixels_slice` (default policy)                                                             | Clean, 0 errors                                                                                      |
| R2  | `SmartLedsWrite::write`                                                                         | Clean, 0 errors                                                                                      |
| R3  | `Poll`                                                                                          | Garbled (steady image); `Error::Transmit` 22–28 per boot during the rainbow, then on every 1 s frame |
| R4  | `with_max_nominal_masked_us(Some(300))` (named `with_max_masked_us` at the time), frame ~376 µs | Nothing sent; only `FrameTooLong`                                                                    |
| R5  | `set_pixels_slice_masked` (bounded)                                                             | Clean, 0 errors                                                                                      |

A second downstream re-bench ran on `c85a61a` (clean tree, no RAM placement, `from_channel_creator`), logging `Ok`, `Transmit` and `ChannelLost` per second.
`c85a61a` replaced `f53c5e8` as the commit under test (an amended commit).

| Run | Call                                | Result                                                                                                                                                  |
|:----|:------------------------------------|:--------------------------------------------------------------------------------------------------------------------------------------------------------|
| R1′ | `set_pixels_slice` (default policy) | 3,888 `Ok`, 0 `Transmit`, 0 `ChannelLost`; clean (SoftAP portal, then station)                                                                          |
| R3′ | `Poll`                              | 8,396 `Ok`, 23 `Transmit`, 0 `ChannelLost`; 1–3 `Transmit` per boot during the ~1 s Wi-Fi connect, about 1 per 400 frames on the portal; visually clean |
| R6  | `Poll`, radio never started         | 366 `Ok`, 0 errors; clean                                                                                                                               |

On a downstream C3 bench with Wi-Fi active, the blocking default policy, `SmartLedsWrite` and `set_pixels_slice_masked` were clean.
`Poll` garbled frames, and R3 could not tell two cases apart: a frame that timed out and kept its channel, and a drain that failed and lost it.
Both returned `Error::Transmit`.
The R1–R5 results apply to `f53c5e8`, and R1′–R6 to `c85a61a`; any later code change is a new binary and needs its own run.
The lost-channel wording in the amendment above (repeated `Error::Transmit`) is superseded by the `ChannelLost` decision below.
Reading the `c85a61a` runs: the drain assumption holds on this bench, and `Poll` keeps its channel.
On `f53c5e8`, R3 could not be split between "timed out, channel kept" and "channel lost", so that run stays ambiguous.

### Decision: `Error::ChannelLost`

- A blocking call whose drain fails returns `Error::ChannelLost`, and so does every later call on that driver.
- `Error::Transmit` now means the frame failed and the driver is usable.
- The async driver never loses its channel.

Reason: consumers must tell "this frame failed, the driver is usable" from "dead until rebuilt".
Returning one variant for both cases made R3 ambiguous, and the caller had no way to decide whether to rebuild.
This is a breaking change to `Error`, which is already `#[non_exhaustive]`.

### RAM placement

- Required evidence exists only for the async `CriticalSection` path, which garbled 12-LED frames with Wi-Fi active until `ESP_HAL_CONFIG_PLACE_RMT_DRIVER_IN_RAM=true` was set.
- The blocking masked path was clean on a downstream C3 bench with Wi-Fi active in three runs at `f53c5e8` (R1, R2 and R5 above), without that setting, after an earlier unbounded-`wait()` run.
- RAM placement is still recommended as margin for radio firmware, but this ADR does not claim it is required for the blocking path.

## Correction: ESP-IDF IRAM-safe setting (2026-11-01)

The "ESP-IDF radio wording" decision recommended `CONFIG_RMT_ISR_IRAM_SAFE=y` for firmware that writes flash.
That recommendation was wrong and is withdrawn.

- It came from reading the ESP-IDF Kconfig and this crate's refill path.
  It was not checked against the esp-idf-hal layer that sits between them.
- A downstream ESP32-C3 bench (ESP-IDF v5.3.3, esp-idf-hal 0.47.0, `f53c5e8`) enabled the setting.
  `Ws2812Rmt::new` then failed with `ESP_ERR_INVALID_ARG` ("on_trans_done callback not in IRAM"), and the OTA image rolled back.
- The cause is in the esp-idf-hal layer: its `TxChannelDriver` always registers the flash-resident `handle_isr` as `on_trans_done` (esp-idf-hal `src/rmt/tx_channel.rs:35-37` and `:216`).
  ESP-IDF rejects a flash-resident `on_trans_done` callback when `CONFIG_RMT_ISR_IRAM_SAFE=y` (`rmt_tx.c:506-508`).
  Both references were verified from source on 2026-11-01.
- With the default `n`, the ring stayed clean through a 1.6 MB OTA download (about 17 s, about 170 frames).
  That is one run with no analyser.

Changes that follow:

- The crate docs say "Do not enable `CONFIG_RMT_ISR_IRAM_SAFE`".
- When the setting is on, the constructor's `anyhow` error names it as a likely cause (see the PR review note below).
  The crate detects the setting at build time through the `esp_idf_rmt_isr_iram_safe` cfg, declared with check-cfg in `build.rs`.
- Upstream follow-up: esp-idf-hal should place `handle_isr` in IRAM, or skip registering it when no async wait is used.
  Tracked in the [feature doc](../features/driver-interchangeability-v1.md).

## PR review (2026-11-01)

A PR review changed three things; the decisions above still hold.

- **Rename:** `with_max_masked_us` is now `with_max_nominal_masked_us`, `max_masked_us()` is now `max_nominal_masked_us()`, and `DEFAULT_MAX_MASKED_US` is now `DEFAULT_MAX_NOMINAL_MASKED_US`.
  Behaviour is unchanged.
  The name now says the ceiling limits the nominal estimate, not a hard interrupts-off bound.
  The sections above are renamed in place where they name the API; the R4 record keeps the name used at the time.
- **Clock rate:** `tx_channel_config_for(source: Rate) -> Result<TxChannelConfig, Error>` derives the RMT divider for the 10 MHz tick from the source rate given to `Rmt::new`.
  It returns `Error::RmtConfig` when no exact divider exists.
  `tx_channel_config()` and `Ws2812Rmt::from_channel_creator` remain the shorthand for an 80 MHz source, and they require `Rmt::new(.., Rate::from_mhz(80))`.
- **ESP-IDF hint:** the constructor's error now names `CONFIG_RMT_ISR_IRAM_SAFE` as a likely cause, not the only one.
  Other invalid configurations can also return `ESP_ERR_INVALID_ARG`, so the hint does not assert the cause.

## Related

- [ADR 005: Dual-HAL Strategy](005-dual-hal-strategy.md)
- [ADR 006: Async Support Strategy](006-async-support.md)
- [ADR 008: Embassy as the Async Runtime](008-embassy-as-async-runtime.md)
- [ADR 009: Long-Frame Policy and Latch Enforcement](009-long-frame-policy-and-latch.md)
- [Feature: Interchangeable esp-hal and ESP-IDF drivers v1](../features/driver-interchangeability-v1.md)
