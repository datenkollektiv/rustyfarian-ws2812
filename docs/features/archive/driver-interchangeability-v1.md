---
gate: none
desk-work: available
---

# Feature: Interchangeable esp-hal and ESP-IDF drivers v1

Requested from downstream project `rustyfarian-rgb-clock` (12 LEDs, ESP32-C3), filed 2026-11-01.
The same 12-LED clock behaved differently on the two drivers: blocking esp-hal writes garbled under Wi-Fi, and the ESP-IDF driver sent frames with no reset interval.

## Goal

One meaning per method name across both drivers.
The decisions are recorded in [ADR 010](../../adr/010-interchangeable-driver-semantics.md), which extends [ADR 009](../../adr/009-long-frame-policy-and-latch.md).

## Decisions

| Decision                                                                                                                                                                                                                                    | Reason                                                                                                                                                | Rejected Alternative                                                                             |
|:--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:------------------------------------------------------------------------------------------------------------------------------------------------------|:-------------------------------------------------------------------------------------------------|
| `LongFramePolicy` applies to the blocking and async esp-hal drivers                                                                                                                                                                         | Blocking long frames garbled under Wi-Fi with unmasked polling                                                                                        | Keep the policy async-only                                                                       |
| Default policy stays `CriticalSection`                                                                                                                                                                                                      | Bounded; same policy as the async path benched clean on the C3 with Wi-Fi at an earlier commit (RMT driver in RAM)                                    | Default to `Reject` (refuses long strips that work today)                                        |
| `LongFramePolicy::Poll`: busy-poll the refill with interrupts live, bounded by the timeout                                                                                                                                                  | Long non-radio strips avoid masked interrupts                                                                                                         | `Await` (executor latency, bounds nothing, [ADR 009](../../adr/009-long-frame-policy-and-latch.md)) |
| `Poll` documented as not radio-safe and opt-in                                                                                                                                                                                              | Radio and other interrupts can delay the refill                                                                                                       | Make `Poll` the default                                                                          |
| Blocking `CriticalSection` polls `TxTransaction::poll()` with a timeout of 2 µs per code plus 1 ms; the blocking call is bounded to about two timeout budgets (the timeout, then the drain), except a dead classic-ESP32 peripheral         | An unbounded masked `wait()` can hold interrupts off indefinitely                                                                                     | Keep the unbounded `wait()`                                                                      |
| Blocking timeout: leave the critical section, drain unmasked for a second budget; if the frame ends, return `Error::Transmit` with the channel kept; if not, drop the transaction                                                           | Bounded; the channel is kept only when the drain completes (esp-hal has no stop-and-return for single-shot TX)                                        | `Error::FrameTooLong` or a new `Timeout` variant                                                 |
| Blocking drain failure: driver has no channel; later calls return `Error::ChannelLost`; rebuild needs `Rmt` from `peripherals.RMT.reborrow()` and the pin via `reborrow()`, otherwise only a reset recovers; rebuild not hardware-tested    | esp-hal 1.2.2 offers no way to return the channel from a single-shot transaction                                                                      | Keep a channel reference across the drop (not possible)                                          |
| `Error::ChannelLost` for a blocking call whose drain fails, and for every later call; `Error::Transmit` means the frame failed and the driver is usable                                                                                     | R3 could not tell "timed out, channel kept" from "channel lost" (both `Transmit`); consumers must tell "this frame failed" from "rebuild"             | Keep one `Transmit` variant for both                                                             |
| Masked-time ceiling: `DEFAULT_MAX_NOMINAL_MASKED_US = 1000` caps the estimated nominal masked time (codes × 1.3 µs); longer `CriticalSection` frames return `FrameTooLong`; on a stalled refill the actual window is bounded by the timeout | Keeps the default masked window near 1 ms for radio firmware                                                                                          | No ceiling (unbounded masked window for long strips)                                             |
| `with_max_nominal_masked_us(Option<u32>)` and `max_nominal_masked_us()`; `None` lifts the ceiling (renamed from `with_max_masked_us` in the 2026-11-01 PR review; the limit is on the nominal estimate, not a hard bound)                   | Long strips can opt out explicitly, with the cost visible                                                                                             | Raise the default to fit 64 LEDs                                                                 |
| `set_pixels_slice_masked` is not capped by the ceiling                                                                                                                                                                                      | It is the explicit masked option; the caller already chose the window                                                                                 | Apply the ceiling to it too                                                                      |
| Async timeout: drop the future after the critical section, stopping the channel                                                                                                                                                             | ESP32 cleanup spins until End, so it must run with interrupts on                                                                                      | Drop inside the critical section                                                                 |
| Frames that fit in channel RAM (including `set_pixel`) keep plain transmit and `wait()`, no timeout                                                                                                                                         | A late poll cannot garble them; a timeout would report false errors                                                                                   | Bound every blocking transmit                                                                    |
| Failed blocking transmit start returns the channel (`(Error, Self)`)                                                                                                                                                                        | The driver is never left without a channel; fixes a loss that predates this change                                                                    | Drop the channel on start failure                                                                |
| `set_pixels_slice_masked` kept and always masks                                                                                                                                                                                             | Benched clean downstream on the C3 with Wi-Fi at an earlier commit (unbounded `wait()` in the critical section); R5 clean at `f53c5e8` (bounded code) | Remove it now (breaks existing callers)                                                          |
| `set_pixels_slice_masked` deprecation deferred, then applied 2026-11-02 (`since = "0.8.0"`); removal deferred                                                                                                                               | The re-benches confirmed the default policy covers its use case; downstream still calls it                                                            | Remove it in 0.8 (breaks existing callers)                                                       |
| `Error` implements `core::error::Error`                                                                                                                                                                                                     | Generic error handling in `no_std` without a copy of the type                                                                                         | Inherent `Display` only                                                                          |
| `tx_channel_config()` returns the required RMT configuration                                                                                                                                                                                | Every consumer copies the same setup today                                                                                                            | Document the setup only                                                                          |
| `Ws2812Rmt::from_channel_creator(creator, pin) -> Result<Self, Error>`                                                                                                                                                                      | One call from creator and pin; `ConfigError` maps to `Error::RmtConfig`                                                                               | Callers keep calling `configure_tx` and `with_pin`                                               |
| `tx_channel_config_for(source: Rate) -> Result<TxChannelConfig, Error>` derives the 10 MHz tick divider from the `Rmt::new` source rate                                                                                                     | Consumers on another source rate need the divider too; `tx_channel_config()` stays the 80 MHz shorthand                                               | Document the 80 MHz requirement only                                                             |
| ESP-IDF: `WS2812_RESET_US = 280` enforced with `next_frame_at`                                                                                                                                                                              | Back-to-back frames have no reset interval                                                                                                            | Append a reset code to every frame (+280 µs per frame)                                           |
| ESP-IDF wait uses `Ets::delay_us`                                                                                                                                                                                                           | Avoids FreeRTOS tick rounding                                                                                                                         | `FreeRtos::delay_ms`                                                                             |
| ESP-IDF radio wording: tolerant of task latency, not hard real time                                                                                                                                                                         | The refill ISR is not IRAM-safe by default in ESP-IDF v5.3.3                                                                                          | Describe the driver as radio-safe                                                                |
| Do not enable `CONFIG_RMT_ISR_IRAM_SAFE`; the constructor's error names the setting as a likely cause, not the only one (detected via the `esp_idf_rmt_isr_iram_safe` cfg)                                                                  | Enabling it makes `Ws2812Rmt::new` fail under esp-idf-hal 0.47.0 (`on_trans_done` not in IRAM); recommendation withdrawn 2026-11-01                   | Recommend it for NVS/OTA firmware (wrong, see ADR 010 correction)                                |
| No `sdkconfig.defaults` change in this repository                                                                                                                                                                                           | `CONFIG_RMT_ISR_IRAM_SAFE` is a per-firmware choice                                                                                                   | Set it in this repository's defaults                                                             |
| Breaking changes ship in the unreleased 0.8                                                                                                                                                                                                 | The release already breaks, so consumers take one migration                                                                                           | Patch release                                                                                    |
| No shared `Ws2812` trait across crates                                                                                                                                                                                                      | `SmartLedsWrite` is already the common seam                                                                                                           | New cross-crate trait                                                                            |

## Constraints

- Masked paths cost about 1.25 µs per pulse code: about 0.36 ms for 12 LEDs and 1.9 ms for 64 LEDs.
- Long interrupts-off windows can break Wi-Fi and ESP-NOW and overflow UART FIFOs.
- The blocking default now masks long frames, so the CHANGELOG entry under `[Unreleased]` must say so.
- Earlier downstream Wi-Fi sign-offs (`set_pixels_slice_masked` on an unbounded `wait()`, and the async `CriticalSection` path with RMT driver in RAM) pin earlier binaries and do not carry over; still open after `f53c5e8`: the async path, other chips, longer strips, the waveform capture, the drop and rebuild path, and the R3 re-run with `ChannelLost`.
- A stopped clock ends the poll loop after 1000 stalled reads with 16 polls per read, which is milliseconds, not sub-millisecond.
- A dead ESP32 peripheral can still spin in `TxGuard::drop`, because classic ESP32 fills the end marker and waits for End; other chips stop the channel immediately.
- The masked-time ceiling makes long `CriticalSection` strips fail with `FrameTooLong` by default, a breaking change for 0.8 users with long strips.
- Stack stays at esp-hal `=1.2.2` and esp-rtos `=0.4.0`; no new dependencies.
- The esp-hal latch wait and critical-section implementation still need the `rt` feature ([ADR 009](../../adr/009-long-frame-policy-and-latch.md)).
- ESP-IDF refill runs in the RMT ISR; the docs must not describe it as hard real time.

## Open Questions

- [x] Deprecate `set_pixels_slice_masked` once the downstream re-bench confirms the blocking default policy? Yes: deprecated 2026-11-02 for 0.8.0, after the f53c5e8/c85a61a re-benches confirmed the default policy; removal is a later decision.
- [x] Sweep the remaining examples to `tx_channel_config()`, or leave them as they are? Swept to `from_channel_creator` by the cross-platform streamlining (`a530c4b`).
- [x] Does `Poll` under Wi-Fi garble as documented? Yes: R3 on `f53c5e8` garbled, with `Error::Transmit` 22–28 per boot and on every 1 s frame.
- [x] Does `Poll` lose the channel under Wi-Fi? No: R3′ on `c85a61a` logged 23 `Transmit` and 0 `ChannelLost` over 8,396 frames, and R6 was clean with the radio never started. The drain assumption holds on this bench.
- [x] Is `CriticalSection` acceptable for 300-LED strips? That is about 9 ms with interrupts masked, against a timeout of about 15 ms. Not by default: the 1000 µs nominal ceiling (32 LEDs) returns `FrameTooLong`; such strips lift the ceiling explicitly or choose `Poll`.
- [x] Can the ESP-IDF crate detect `CONFIG_RMT_ISR_IRAM_SAFE` at build time and warn when it is off? Answered: yes for the on case. The crate reads the setting through the `esp_idf_rmt_isr_iram_safe` cfg, declared with check-cfg in `build.rs`, and the constructor's error names the setting when it is on. The default-off case needs no warning.
- [x] Upstream follow-up (esp-idf-hal): place `handle_isr` in IRAM, or skip registering it as `on_trans_done` when no async wait is used. Until then, `CONFIG_RMT_ISR_IRAM_SAFE` cannot be enabled with `Ws2812Rmt::new`. Moved to the roadmap (Long term) on 2026-11-03.
- [x] `set_pixels_slice(&[])` sends a bare end marker, while `SmartLedsWrite::write` with an empty iterator sends nothing; both drivers should agree. Aligned 2026-11-02: empty input is a no-op returning `Ok(())` in `set_pixels_slice` (blocking and async, both crates) and the deprecated `set_pixels_slice_masked`, matching `SmartLedsWrite`; a bare end marker latches no data and is indistinguishable from silence on the strip.
- [x] No runtime tests cover the `PollBudget` and latch logic. The esp-hal host tests cannot build on macOS (an `esp32c6` Mach-O `link_section` error), and the ESP-IDF crate is `espidf`-only. Should the clock be injectable so the logic can be tested on the host? Moved to the roadmap (Mid term) on 2026-11-03.
- [x] Is esp-hal's `Instant::now` inlined into the RAM-placed poll loops? Moved to [long-frame hardware validation v1](../long-frame-hardware-validation-v1.md) as a desk check on 2026-11-03.
- [x] Pre-existing `just check-ide` failure (E0463, no `std` for `riscv32imac`), reproduced on a clean HEAD export. Separate issue, filed as a bug on 2026-11-03.

## State

- [x] Design decided and recorded in [ADR 010](../../adr/010-interchangeable-driver-semantics.md) (Accepted 2026-11-01)
- [x] [ADR 009](../../adr/009-long-frame-policy-and-latch.md) points to ADR 010
- [x] esp-hal: blocking routing, `Poll`, bounded masked transmit, `core::error::Error`, `tx_channel_config()`, `from_channel_creator` (landed in ea0cf39, 2026-11-01)
- [x] ESP-IDF: `WS2812_RESET_US` latch with `next_frame_at` (landed in ea0cf39, 2026-11-01)
- [x] Documentation: crate READMEs, rustdoc, CHANGELOG `[Unreleased]`
- [x] Checks: `just verify`, `just test`, `just check-hal-xtensa`, example builds (re-run 2026-11-02, all PASS)
- [x] Hardware re-bench of `f53c5e8` on a downstream ESP32-C3 (rev v0.3, 12 LEDs on GPIO4, Wi-Fi active, RMT not in RAM), 2026-11-01:
  - [x] R1 `set_pixels_slice` default policy: clean, 0 errors
  - [x] R2 `SmartLedsWrite::write`: clean, 0 errors
  - [x] R3 `Poll`: garbled, `Error::Transmit` 22–28 per boot and on every frame (ambiguous before `ChannelLost`)
  - [x] R4 `with_max_nominal_masked_us(Some(300))` (then `with_max_masked_us`) on a ~376 µs frame: nothing sent, `FrameTooLong` only
  - [x] R5 `set_pixels_slice_masked`: clean, 0 errors
  - [x] Re-run R3 on the `ChannelLost` build: R3′ on `c85a61a` (see below)
  - [x] Re-run R4 with the ceiling lifted or raised: moved to [long-frame hardware validation v1](../long-frame-hardware-validation-v1.md) on 2026-11-03
- [x] Not covered here: the async path, other chips, longer strips, the waveform capture, the drop and rebuild path; moved to [long-frame hardware validation v1](../long-frame-hardware-validation-v1.md) on 2026-11-03
- [x] `c85a61a` replaced `f53c5e8` (amended commit) and carries the `ChannelLost` build; the R1′, R3′ and R6 runs on it are recorded in ADR 010
  - [x] R1′ `set_pixels_slice` default: 3,888 `Ok`, 0 `Transmit`, 0 `ChannelLost`, clean
  - [x] R3′ `Poll`: 8,396 `Ok`, 23 `Transmit`, 0 `ChannelLost`, visually clean
  - [x] R6 `Poll`, radio never started: 366 `Ok`, 0 errors, clean

Results apply to the named commit only; any later code change is a new binary and needs its own run.

## Sources

- [`crates/rustyfarian-esp-hal-ws2812/src/lib.rs`](../../../crates/rustyfarian-esp-hal-ws2812/src/lib.rs) — driver under change
- [`crates/rustyfarian-esp-idf-ws2812/src/lib.rs`](../../../crates/rustyfarian-esp-idf-ws2812/src/lib.rs) — driver under change
- esp-hal 1.2.2 `src/rmt.rs:1521` — non-consuming `TxTransaction::poll()`
- ESP-IDF v5.3.3 Kconfig — `CONFIG_RMT_ISR_IRAM_SAFE` defaults to `n`
- esp-idf-hal 0.47.0 `src/rmt/tx_channel.rs:35-37`, `:216` — `handle_isr` registered as `on_trans_done`
- ESP-IDF `components/esp_driver_rmt/src/rmt_tx.c:506-508` — rejects a flash-resident `on_trans_done` when IRAM-safe is on

## Session Log

- 2026-11-01 — Review of `2931479`: blocking timeout changed to drain-then-drop and the masked-time ceiling added (ADR 010 amendment)
