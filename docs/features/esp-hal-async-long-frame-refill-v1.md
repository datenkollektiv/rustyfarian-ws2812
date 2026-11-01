---
gate: hardware
gate-note: Waveform capture per the maintenance-plan long-frame release gate on ESP32-C3
desk-work: available
---

# Feature: Deadline-free async transmit for long frames in `rustyfarian-esp-hal-ws2812` v1

Requested from the downstream `rustyfarian-rgb-clock` project (12 LEDs, ESP32-C3), filed 2026-10-30.
Reproduced independently by a second downstream project (64 LEDs, ESP32-C3, ESP-NOW), 2026-10-31.
`Ws2812Rmt<'d, Async, N>` garbles any frame longer than one channel's RAM, because esp-hal 1.2.2 refills TX RAM in `TxFuture::poll`, not in the RMT ISR.

## Root cause (verified in esp-hal 1.2.2 source, 2026-10-31)
- The async ISR only clears the interrupt and wakes the task (`src/rmt.rs:2345-2372`).
- The refill runs in `TxFuture::poll` (`src/rmt.rs:1986-1993`), so it waits for the executor.
- The threshold is half the channel RAM (`src/rmt.rs:2183`): 24 codes ≈ 30 µs before the hardware wraps.
- A late refill resends stale codes, which shifts the picture.
- The blocking `wait()` refills in a busy loop (`src/rmt.rs:1527-1535`), and `get_tx_status` reads `int_raw` (`src/rmt.rs:2578`), so polling works with interrupts masked.
- `channel_ram_size` is 48 codes on C3/C6/S3 and 64 on ESP32 (esp-metadata-generated 0.5.3).

## Decisions
| Decision                                                                                            | Reason                                                                                          | Rejected Alternative                                          |
|:----------------------------------------------------------------------------------------------------|:------------------------------------------------------------------------------------------------|:--------------------------------------------------------------|
| Frames > `CHANNEL_RAM_SIZE` codes: busy-poll the future in a critical section                       | Same deadline-free loop as blocking `wait()`                                                    | Await and hope the executor is fast                           |
| Frames that fit stay truly async                                                                    | No refill, so no deadline                                                                       | Critical section for every frame                              |
| `LongFramePolicy`: `CriticalSection` (default) or `Reject`                                          | Radio firmware can refuse masked interrupts                                                     | Always on, no opt-in                                          |
| No `Await` policy variant ([ADR 009](../adr/009-long-frame-policy-and-latch.md))                    | It garbles and bounds nothing                                                                   | `Await` variant                                               |
| Threshold `esp_hal::rmt::CHANNEL_RAM_SIZE` (one block)                                              | Driver cannot read the configured `memsize`                                                     | Constructor `memsize` argument                                |
| `Error` is `#[non_exhaustive]`; `Error::FrameTooLong` for `Reject`                                  | Refused frames must be reported, not dropped                                                    | Reuse `Error::Transmit`                                       |
| Remove the policy branch once esp-hal refills in the ISR                                            | No policy needed when the refill is deadline-free                                               | New `set_pixels_slice_cs` method                              |
| `critical-section` as direct optional dependency under `async`                                      | esp-hal already provides the impl                                                               | Unsafe register-level interrupt masking                       |
| Drop the future after the critical section on timeout                                               | ESP32 cleanup spins until End, so IRQs must be on                                               | Drop inside the critical section                              |
| Latch: encoded in the signal on the async await path (`LATCH_END_CODE`)                             | Free there: the await covers it, no CPU wait                                                    | Busy-wait on every path                                       |
| Latch: busy-wait ≤ 280 µs before the next frame on the CS and blocking paths                        | Encoding there adds 280 µs of interrupts-off or CPU                                             | In-signal reset code (+280 µs per frame)                      |
| Cancellation guard (`NextFrameStamp`) schedules the latch on every exit                             | A cut-short frame must not make the next one a continuation                                     | Record end time only on completion                            |
| Return `Error::Transmit` on timeout                                                                 | Existing variant; driver stays reusable                                                         | New `Timeout` variant                                         |
| Ship in the next 0.x minor                                                                          | Timing of long frames changes; `Error` is breaking                                              | Patch release                                                 |
| Radio firmware builds with `ESP_HAL_CONFIG_PLACE_RMT_DRIVER_IN_RAM=true` (documented, not enforced) | Bench: garbled with Wi-Fi active, clean with RAM placement; the crate cannot detect the setting | Crate-side check (impossible: esp-hal-internal cfg)           |
| RAM-placed poll loop (`poll_to_completion`, `#[esp_hal::ram]`), clock read every 16 polls           | Keeps the loop tight; timeout overshoot a few µs                                                | Read the clock on every poll (slower)                         |
| Blocking masked transmit `set_pixels_slice_masked` offered as an option                             | Validated with Wi-Fi on the C3; same interrupts-off cost as CriticalSection                     | Retract blocking as a fallback (plain blocking stays exposed) |
| Poll-gap diagnostics (`long-frame-diagnostics`) removed before release                              | Refill runs inside `poll`; 77 µs gaps had clean frames                                          | Ship as a feature (slows every poll, false alarms)            |

## Constraints
- Stack stays at esp-hal =1.2.2, esp-rtos =0.4.0.
- Interrupts are off for about 1.25 µs per code: 0.36 ms for 12 LEDs, 1.9 ms for 64 LEDs; the docs must state this.
- Long interrupts-off windows can break Wi-Fi/ESP-NOW and overflow UART FIFOs; keep the strip length within about 1 ms or choose `Reject`.
- The `critical-section` impl and the latch timer both come from esp-hal's `rt` feature; without `rt` the async driver fails to link.
- The latch timer is started by `esp_hal::init`, so safe code cannot build the driver without it; only an `unsafe` setup that skips `init` stops it, and then the busy-wait gives up after about 1000 unchanged reads.
  Short async frames are unaffected because their latch is in the signal.
- Short async frames complete about 280 µs later (yielding), since the latch is encoded in their last slot.
- The critical section restores the caller's interrupt state; if the caller already runs masked, the ESP32 cleanup spin and the latch wait run masked too.
- On dual-core ESP32/S3 the other core spins on any `critical_section::with` for the whole frame; the docs must say so.
- Dropping the future mid-frame stops the channel (`src/rmt.rs:1428-1440`); on ESP32 that spin runs after the critical section.
- `set_pixel` and `AsyncStatusLed` (25 codes) keep their current truly async path.
- The blocking driver changes only by the latch wait; the ESP-IDF driver stays unchanged.
- All checks go through `just`; `dependency-manager` and `code-reviewer` sign off.

## Open Questions
- [x] Pending RMT interrupt after the critical section is harmless: the ISR only unlistens and wakes; raw status stays for polling (source), and C3 phase 3 ran await frames right after CS frames, 2 cycles PASS (2026-10-31).
- [ ] File the esp-hal issue asking for ISR refill (draft for the maintainer to post)?

## State
- [x] Design approved
- [x] Core implementation
- [x] Checks passing (`just check-hal`, `check-hal-c3`, `check-hal-xtensa`, new `check-hal-async`, `clippy-hal-async`, `verify`, `deny`)
- [x] Review follow-up implemented
- [ ] Hardware-validated per the maintenance-plan long-frame release gate (`hal_c3_frame_sweep_async`)
- [x] Documentation updated (lib.rs, crate README, CHANGELOG, lore, ADR 008)

## Sources
- [esp-hal #1779 — Async RMT Tx not working with >64 PulseCodes](https://github.com/esp-rs/esp-hal/issues/1779) — maintainer: refill "very time critical", suggests ISR refill
- [esp-hal #4049 — RMT: Support wrapping everywhere](https://github.com/esp-rs/esp-hal/pull/4049) — added waker-based async TX wrap, closed #1779
- [esp-hal #4126 — RMT: refactor copies to/from channel RAM](https://github.com/esp-rs/esp-hal/pull/4126) — introduced `RmtWriter`
- [esp-hal #4604 — RMT: Use new Encoder trait for Tx data](https://github.com/esp-rs/esp-hal/pull/4604) — open rewrite of the TX write path
- [esp-hal #3930 — RMT driver tracking issue](https://github.com/esp-rs/esp-hal/issues/3930) — no entry for ISR refill
- [esp-hal #2277 — Async RMT seems to produce incorrect output](https://github.com/esp-rs/esp-hal/issues/2277) — unrelated cause (starved task)
- [crates.io esp-hal versions](https://crates.io/api/v1/crates/esp-hal/versions) — 1.2.2 is latest as of 2026-10-31

## Session Log
- 2026-10-31 — Feature doc created from the downstream request; root cause verified against esp-hal 1.2.2 source
- 2026-10-31 — Independent debugger + embedded-systems review: root cause confirmed, no blocker; added bounded spin and doc constraints
- 2026-10-31 — Implemented on branch `fix/esp-hal-async-long-frame-refill`; new `check-hal-async` CI gate; all checks green; hardware run pending
- 2026-10-31 — Review follow-up: drop after critical section, `LongFramePolicy`, latch enforcement, ADR 009; hardware gate pending
- 2026-10-31 — Review follow-up implemented: P1 drop-after-CS, `LongFramePolicy`, latch with stalled-timer detection, `doc-hal`; all checks green; waveform gate pending
- 2026-10-31 — Partial gate on ESP32-C3 (CH340 board), sweep ELF sha256 a1b9903ad77e67d4…: 2 cycles PASS in the serial log; no analyser, radio load or RAM-placement runs yet
- 2026-10-31 — Review follow-up 2: blocking fallback for long frames retracted; encoded latch on the async await path, busy-wait latch on the CS and blocking paths; cancellation guard (`NextFrameStamp`); interrupt-state and timer-start precision; hardware gate still open
- 2026-10-31 — C3 (CH340) sweep ELF sha256 495cbee68b1d47dc…: 3 cycles PASS; encoded latch held (1-LED back-to-back 342–345 µs ≥ 308), cancellation guard forced the gap (619–633 µs); analyser, radio, RAM-placement, other chips open
- 2026-11-01 — Hardening pass (stalled clock in CS loop, named transmit local, ceil + path-aware minimums); C3 ELF sha256 6ab25cf1857fb460…: 5 cycles PASS, captured via espflash monitor
- 2026-11-01 — C3 12-LED ring, `hal_c3_rainbow_comet_async` ELF sha256 3f8c3ec7c2f83ad6…: ~600 frames, 0 errors, send 390–428 µs, loop max 30–31 ms; visually smooth (freezes traced to resets)
- 2026-11-01 — C3 load test `hal_c3_comet_under_load_async` ELF sha256 e4942a103c963b93…: 2 passes of 4 modes, 0 errors, visually clean; 12-LED frames raise worst interrupt gap from ~118 µs to ~455–478 µs
- 2026-11-01 — Maintainer visual check: no issues in hal_c3_pulse, frame sweep, rainbow comet or comet under load on the C3 12-LED ring; only freezes were resets (RST, monitor connect)
- 2026-11-01 — rgb-clock reports CriticalSection garbles with Wi-Fi (b2b2022). Cold-cache test here (32 KB flash sweep before each frame, ELF fe55e33dc03db3d4…): send 440→865 µs, 0 errors, visually clean — cache alone ruled out
- 2026-11-01 — Review follow-up 3: radio finding recorded (downstream bench: garbles with Wi-Fi active, clean with `ESP_HAL_CONFIG_PLACE_RMT_DRIVER_IN_RAM=true`; masked blocking clean; radio contention is a hypothesis); blocking masked option documented; `long-frame-diagnostics` added; poll-gap margin not yet measured; hardware gate open
- 2026-11-01 — Downstream Wi-Fi bench on 3a1d3f8 (C3, 12 LEDs): CriticalSection clean with only our loop in RAM (F1) and with esp-hal RMT in RAM (F2); set_pixels_slice_masked clean (F3); margin and waveform unmeasured
- 2026-11-01 — T8 on 735b506 (downstream Wi-Fi bench): all runs clean; longest loop pass 8–9 µs (no radio), 27–47 µs (radio, RMT in RAM), 76–77 µs (radio, RMT in flash); max_poll_gap_us cannot bound the refill deadline (refill happens inside a poll)
- 2026-11-01 — `long-frame-diagnostics` removed before release (gaps between polls cannot bound the refill); margin evidence is downstream visual validation with Wi-Fi on `3a1d3f8` (F1–F3) plus the open analyser capture
