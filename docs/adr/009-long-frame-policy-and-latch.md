# ADR 009: Long-Frame Policy and Latch Enforcement

## Status

Accepted (2026-10-31)

## Context

The feature doc [`esp-hal-async-long-frame-refill-v1`](../features/esp-hal-async-long-frame-refill-v1.md) describes the async workaround for frames longer than one RMT channel RAM block.
esp-hal 1.2.2 refills TX RAM in `TxFuture::poll`, not in the RMT ISR, so an async frame that outlives the refill deadline can be corrupted.
The workaround (commit `8b592bd`) busy-polls the transmit future inside `critical_section::with` and drops it on timeout.

An external review of that commit raised three points, all verified against the esp-hal 1.2.2 source:

1. **Timeout does not bound interrupts-off time on ESP32.**
   The timed-out future drops inside the critical section.
   On the classic ESP32, `rmt.has_tx_immediate_stop` is `false`, so `TxGuard::drop` spins until the End event with interrupts still masked (`src/rmt.rs:1428-1440`, `3106-3113`).
2. **No reset interval between frames.**
   `PulseCode::end_marker()` is `Self(0)` (`src/rmt.rs:434`), and nothing enforces the WS2812 reset low time (at least 280 µs for WS2812B) between two transmits.
   This predates the workaround and affects every path, blocking included.
3. **Masking interrupts is not a deadline guarantee.**
   Masking removes executor latency from the refill path, but not flash-cache misses, cross-core flash stalls, or interrupts that a critical section cannot mask.
   The earlier docs claimed the output "no longer garbles", which overstated the fix.

Frames that fit one RAM block are unaffected, and so are `set_pixel` and `AsyncStatusLed`.
The interrupts-off cost is about 1.25 µs per code: about 0.36 ms for 12 LEDs and 1.9 ms for 64 LEDs.

## Decision

**Expose a configurable long-frame policy on the async driver, with the critical-section path as default; enforce the latch interval in both drivers, encoded in the signal on the async await path and as a busy-wait elsewhere.**

### Long-frame policy

- `LongFramePolicy` is an `#[non_exhaustive]` enum with two variants: `CriticalSection` (default) and `Reject`.
- `CriticalSection` keeps the busy-poll path, bounded by a timeout, with the future dropped after the critical section.
- `Reject` returns `Error::FrameTooLong` before any transmit, for firmware that must never mask interrupts or risk corrupt output.
- `Ws2812Rmt<'d, Async, N>::with_long_frame_policy` sets the policy.
- `Error` becomes `#[non_exhaustive]`, which is a breaking change that lands in the same unreleased minor as the async feature split.

No `Await` (plain `transmit().await`) variant ships now.
It is the known-garbling mode and bounds nothing, so offering it as a policy would endorse the behaviour the review rejected.
It can be added without a breaking change once esp-hal refills TX RAM in the ISR.

The blocking driver is not offered as a fallback for long frames.
Its polling refill has the same deadline, and radio or other interrupts can delay it.

### Drop after the critical section

The future is held in a pinned slot declared before `critical_section::with`.
On timeout, the closure returns `Err` and the slot drops after the critical section ends, so the ESP32 cleanup spin runs after the section.
The critical section restores the caller's interrupt state, so the cleanup runs with interrupts enabled unless the caller already masks them.

### Latch enforcement

- `pub const WS2812_RESET_US: u32 = 280` is not configurable, because a shorter reset only invites corrupted frames.
  A clamped `with_reset_us` may follow later.
- **Async await path (frame fits in channel RAM):** the last slot of the frame is replaced by `LATCH_END_CODE`, a 280 µs low code followed by the end marker.
  The latch is then on the wire when the `await` completes, so nothing busy-waits.
  Such frames complete about 280 µs later, while yielding.
- **Critical-section path and blocking driver:** the frame ends with a plain end marker, and the driver stores the earliest start of the next frame (`next_frame_at`).
  Before the next frame it busy-waits only if that instant has not passed, with the caller's interrupt state.
  Encoding the latch there would add 280 µs of interrupts-off time per long frame, or 280 µs of CPU time per blocking frame even when frames are spaced.
- **Clock assumption:** the wait and the critical-section timeout read esp-hal's timer, which `esp_hal::init` starts; that call requires the `rt` feature.
  In safe code the driver cannot be built without it.
  Only an `unsafe` setup that skips `init` leaves the timer stopped; the wait then stops after about 1000 unchanged reads instead of hanging, and the await path is unaffected because its latch is in the signal.

### Cancellation guard

A `NextFrameStamp` drop guard is created before the transmit future and dropped after it.
It records `next_frame_at` on every exit.
When the frame completed with its latch on the wire, the stamp is the current time.
Otherwise, including when the future is dropped mid-frame, it is the current time plus 280 µs.
A cancelled transmit therefore still schedules the latch wait, and the next valid frame is never read as a continuation of the cut-short one.

### Rejected alternative: in-signal reset code everywhere

Appending a low `PulseCode` of 280 µs before the End marker on every path would be clock-independent.
It adds 280 µs to every frame.
On the critical-section path it raises interrupts-off time for a 12-LED frame from about 0.36 ms to about 0.64 ms, and on the blocking path it adds CPU time to every frame.
That cost lands where interrupts are already masked or the CPU is already busy, so the encoded latch is used only on the await path, where it is free.
The timestamp wait remains the mechanism for the critical-section and blocking paths.

## Consequences

### Positive

- The timeout bounds interrupts-off time on every chip, including ESP32.
- Callers who cannot tolerate masked interrupts or corrupt output have a `Reject` policy that fails loudly.
- Consecutive frames respect the WS2812 reset interval in both drivers, and short async frames do so without any CPU wait.
- A cancelled transmit cannot cause the next frame to be read as a continuation.
- The docs claim only what the driver enforces.

### Negative

- `Error` is now `#[non_exhaustive]`, so downstream `match` expressions need a wildcard arm.
- The latch wait needs esp-hal's `rt` feature to take effect, on every path that busy-waits.
- Short async frames complete about 280 µs later, which caps the achievable frame rate for very short frames.
- Long and blocking frames that follow each other closely busy-wait up to 280 µs before the next frame.

### Residual risks

- **Flash-cache misses** can still stall the refill path.
  `ESP_HAL_CONFIG_PLACE_RMT_DRIVER_IN_RAM` is the mitigation to measure.
- **Flash stalls caused by the other core** on ESP32 and ESP32-S3 can stall the refill path.
- **High-priority Xtensa interrupt levels** are not verified to be masked by `critical_section`.
- **If the caller already runs masked** (inside a critical section or a high-priority interrupt executor), the cleanup spin and the latch wait run masked too.
- **Hardware behaviour** is not proven by unit tests, which cannot run on the host.
  The waveform capture in the maintenance-plan long-frame release gate, using `hal_c3_frame_sweep_async`, is the evidence.

## Related

- [ADR 006: Async Support Strategy](006-async-support.md)
- [ADR 008: Embassy as the Async Runtime](008-embassy-as-async-runtime.md)
- [Feature: Deadline-free async transmit for long frames](../features/esp-hal-async-long-frame-refill-v1.md)
