---
gate: none
desk-work: available
---

# Feature: Cross-platform API streamlining v1

Follow-up to [driver interchangeability v1](driver-interchangeability-v1.md), from a codebase investigation on 2026-11-03.
ADR 010 aligned the runtime behaviour of the two ESP drivers; examples, error types, the AVR crate and shared constants were left out.

## Decisions

| Decision                                                                            | Reason                                                                                          | Rejected Alternative              |
|:------------------------------------------------------------------------------------|:------------------------------------------------------------------------------------------------|:----------------------------------|
| Breaking changes ship in the unreleased 0.8                                         | 0.8 already breaks; consumers take one migration                                                | Spread over 0.8 and 0.9           |
| No shared `Ws2812` trait across crates                                              | `SmartLedsWrite` is the common seam ([ADR 010](../adr/010-interchangeable-driver-semantics.md)) | New cross-crate trait             |
| Mechanical items first (examples sweep, error impls, alias)                         | No hardware behaviour change; most of the visible friction                                      | API design items first            |
| Examples use `from_channel_creator`; `hal_c6_rainbow_comet` keeps the two-step form | One call; one example still shows `tx_channel_config()`                                         | Two-step form everywhere          |
| `SpiError` keeps `source()` as `None`                                               | Its `Display` already embeds the encode error                                                   | Return the encode error as source |

## Constraints

- The pure crates and the AVR crate must keep building on the pinned `nightly-2025-04-27` (rust-version 1.88).
- `bunting`, `ferriswheel` and `pennant` stay `no_std` and allocation-free.
- Timing changes to an ESP driver invalidate earlier hardware sign-offs for that driver.
- No new dependencies.

## Open Questions

- [ ] Should the ESP-IDF driver return a typed `Error` instead of `anyhow::Error`, matching esp-hal?
- [ ] Should the AVR drivers gain `set_pixel` / `set_pixels_slice`, or keep `write` as their only send method?
- [ ] Should the AVR drivers implement `pennant::StatusLed`?
- [ ] Should `SmartLedsWrite` stay opt-in on AVR, while both ESP crates always implement it?
- [ ] Which single name should the smart-leds features use (`smart-leds-trait`, `smart-leds-compat`, `smart-leds`)?
- [ ] Should WS2812 pulse timings and `WS2812_RESET_US` move to `bunting`, given esp-hal uses 400 ns T0H and ESP-IDF 350 ns?
- [ ] Should ESP-IDF `new()` pick the RMT block size per chip, given `idf_esp32_rainbow` runs on classic ESP32 with 48 symbols?
- [ ] Should the justfile chip shortcuts follow one driver-naming pattern (today `-esp32` is IDF, `-c3`/`-c6` are HAL)?

## State

- [ ] Design approved
- [x] Sweep the 19 remaining esp-hal examples to `from_channel_creator` (compile-verified on all chips; not yet flashed)
- [x] Implement `core::error::Error` for `EffectError`, `PulseEffectError`, `SpiEncodeError`, `SpiError` and `BitBangError`
- [x] Drop the `.map_err(anyhow!(…))` workarounds in the IDF examples once the error impls land
- [x] Remove the deprecated `WS2812RMT` alias (removal was promised for 0.7.0)
- [ ] Extract one RGB-to-GRB-bytes helper for the three copies in the ESP-IDF driver
- [ ] Change `pennant::PulseEffect::update` to take `RGB8` (slipped from 0.7.0)
- [ ] Add a justfile shortcut for `idf_c3_pulse`
- [x] Flash spot checks at `a5f6b72`; ESP32 and S3 stay compile-verified only
  - [x] `hal_c3_pulse_async` on ESP32-C3 rev v0.3, GPIO4: boots, first LED pulses blue as designed (`AsyncStatusLed` drives one LED); ELF SHA-256 `f9fe0744…` unchanged by the later doc fix
  - [x] `hal_c6_fire` on ESP32-C6 rev v0.1, GPIO18: boots, flame renders correctly; ELF SHA-256 `7c7d94b3…`
  - [x] `hal_c6_pulse_async` on ESP32-C6 rev v0.1, GPIO18: boots, first LED pulses blue as designed; ELF SHA-256 `f4db6164…`
  - [x] `hal_c6_rainbow_comet_async` on ESP32-C6 rev v0.1, GPIO18 (extra): comet renders cleanly, frames logged without errors; ELF SHA-256 `905e9cf9…`
- [x] Fix the `hal_c3_pulse_async` module doc, which promised a pulse on the whole 12-LED ring
- [x] Same fix for `hal_c6_pulse_async` (ELF unchanged) and `hal_esp32_pulse_async` (image changed via LLVM symbol hashes; never flashed, so no sign-off lost)
- [ ] Point `docs/embassy-task-spawning-patterns.md` at `from_channel_creator` (stale pre-1.1 `configure_tx` snippets)
- [ ] Resolve the open questions above and implement the accepted ones
- [ ] `just verify` and `just test` pass; CHANGELOG `[Unreleased]` updated

## Session Log

- 2026-11-03 — Feature doc created from the cross-platform streamlining investigation
- 2026-11-03 — Mechanical batch done: error impls, example sweep, `map_err` cleanup, alias removal; flash spot checks pending
- 2026-11-03 — Review of `a5f6b72`: no blocker; C3 module doc fixed (binary unchanged), C6 spot checks waived
- 2026-11-03 — `hal_c6_fire` flashed at `a5f6b72` on ESP32-C6: clean; `hal_c6_pulse_async` flashed too: clean; all planned spot checks done, plus `hal_c6_rainbow_comet_async`
- 2026-11-03 — Whole-ring wording fixed in the C6 and ESP32 async pulse docs
