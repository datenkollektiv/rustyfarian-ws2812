---
gate: none
desk-work: available
---

# Feature: Long-frame hardware validation v1

Split on 2026-11-03 from [driver interchangeability v1](driver-interchangeability-v1.md) and [deadline-free async transmit v1](esp-hal-async-long-frame-refill-v1.md), whose design and implementation are done.
This doc holds only the hardware evidence both still lacked.

## Goal

Hardware evidence, pinned to one binary each, that the long-frame paths of `rustyfarian-esp-hal-ws2812` behave as [ADR 009](../../adr/009-long-frame-policy-and-latch.md) and [ADR 010](../../adr/010-interchangeable-driver-semantics.md) document.

## Decisions

| Decision | Reason | Rejected Alternative |
|:--|:--|:--|
| Re-run R1, R2, R3 and R5 on the current tree | The `f53c5e8` and `c85a61a` benches pin older binaries; T0H moved to 300 ns since | Carry the old results over |
| ESP32-C3 is the reference chip; the C6 joins if available | The maintenance-plan release gate names both; downstream projects run the C3 | Every chip and strip length |
| `CriticalSection` on 300-LED strips needs no new run | The 1000 µs ceiling (32 LEDs) refuses such frames by default; `Poll` or an explicit ceiling lift is the documented route | Bench a 300-LED strip |
| The Wi-Fi items (R1–R5, async under Wi-Fi) move to a roadmap follow-up; the doc closes on the in-repo items | They run in a downstream project; decided 2026-11-07 for v0.9.0 | Hold the doc open for them |
| The logic-analyser captures move to a roadmap follow-up; the serial gates count for v0.9.0 | Decided 2026-11-07; both frame sweeps passed on serial (C3 3 cycles, C6 12 cycles) | Hold the release for the captures |

## Constraints

- A sign-off is valid only for the binary that ran; record the ELF SHA-256 and the commit for every run.
- The waveform capture follows the long-frame async release gate in `maintenance-plan.md`.
- Radio runs (Wi-Fi, ESP-NOW) stay downstream; this repository has no radio examples.

## Open Questions

- [ ] Is the drop-and-rebuild path after `Error::ChannelLost` worth forcing on hardware, or does the "rebuild not hardware-tested" caveat stay? Carried to the roadmap unanswered; the caveat stays for v0.9.0

## State

The hardware items below no longer gate this feature.
They run once, in the final hardware pass after the [ADR 012](../../adr/012-shared-logic-ladder.md) retrofit and before the release, on the binary that ships.

- [x] Desk check: `objdump` the release `hal_c3_frame_sweep_async` ELF and confirm `Instant::now` is inlined into the RAM-placed poll loops (no call into flash) — 2026-11-07, all four chips, no new flash callees vs v0.8.0
- [ ] Downstream C3 Wi-Fi re-bench on the current tree: R1 default `set_pixels_slice`, R2 `SmartLedsWrite::write`, R3 `Poll`, R5 deprecated `set_pixels_slice_masked` — moved to the roadmap
- [ ] R4 with the ceiling lifted (`with_max_nominal_masked_us(None)`): the frame is sent — moved to the roadmap
- [ ] Async path under Wi-Fi on the C3 (default `CriticalSection`) — moved to the roadmap
- [x] `hal_c3_frame_sweep_async` release gate, serial half: three clean loops on `88e63c6` (`aebb6f0f…`); the GPIO4 capture moved to the roadmap
- [x] Same release gate on the ESP32-C6 (GPIO18) with `hal_c6_frame_sweep_async`, serial half: twelve clean loops on `88e63c6` (`2f76c5d8…`); the capture moved to the roadmap
- [x] Record results in ADR 010 and the CHANGELOG `[Unreleased]` entries that still mark validation as pending

## Session Log

- 2026-11-03 — Created from the leftover hardware gates of driver interchangeability v1 and deadline-free async transmit v1; both archived
- 2026-11-04 — Added `hal_c6_frame_sweep_async`, a port of the C3 sweep, so the C6 item can run
- 2026-11-04 — Hardware gate removed; the bench list moves into the final hardware pass after the ADR 012 retrofit
- 2026-11-07 — Wi-Fi items deferred to the roadmap; the in-repo items run in the v0.9.0 hardware pass
- 2026-11-07 — Logic-analyser captures deferred to the roadmap; serial gates passed on the release binaries
- 2026-11-07 — Results recorded in ADR 010 and the CHANGELOG; archived with the v0.9.0 hardware pass
