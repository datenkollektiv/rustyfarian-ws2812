---
gate: none
desk-work: available
---

# Feature: Cross-platform API streamlining v1

Follow-up to [driver interchangeability v1](driver-interchangeability-v1.md), from a codebase investigation on 2026-11-03.
ADR 010 aligned the runtime behaviour of the two ESP drivers; examples, error types, the AVR crate and shared constants were left out.

## Decisions

| Decision                                                                                                              | Reason                                                                                                                                                        | Rejected Alternative                                |
|:----------------------------------------------------------------------------------------------------------------------|:--------------------------------------------------------------------------------------------------------------------------------------------------------------|:----------------------------------------------------|
| Breaking changes ship in the unreleased 0.8                                                                           | 0.8 already breaks; consumers take one migration                                                                                                              | Spread over 0.8 and 0.9                             |
| No shared `Ws2812` trait across crates                                                                                | `SmartLedsWrite` is the common seam ([ADR 010](../adr/010-interchangeable-driver-semantics.md))                                                               | New cross-crate trait                               |
| Mechanical items first (examples sweep, error impls, alias)                                                           | No hardware behaviour change; most of the visible friction                                                                                                    | API design items first                              |
| Examples use `from_channel_creator`; `hal_c6_rainbow_comet` keeps the two-step form                                   | One call; one example still shows `tx_channel_config()`                                                                                                       | Two-step form everywhere                            |
| `SpiError` keeps `source()` as `None`                                                                                 | Its `Display` already embeds the encode error                                                                                                                 | Return the encode error as source                   |
| ESP-IDF driver returns a typed `Error` (non-exhaustive, `Copy`, `Eq`); `anyhow` becomes a dev-dependency              | Consumers can match the failure kind; `?` into `anyhow` and `.context()` keep working; matches esp-hal                                                        | Keep `anyhow::Error`                                |
| AVR: `write` renamed to `set_pixels_slice`, `set_pixel` added, `write` kept as a deprecated alias                     | One name per behaviour on all three driver crates; existing AVR code compiles with a warning                                                                  | Additive only; keep `write` only                    |
| AVR drivers implement `pennant::StatusLed` behind an opt-in `pennant` feature                                         | Orphan rule forces the impl into the crate; `pennant` is an internal crate, so no new external dependency                                                     | No impl (consumers newtype)                         |
| `SmartLedsWrite` stays opt-in on AVR; the feature joins the avr-none CI checks                                        | The AVR impl costs a 768-byte stack buffer on a 2 KB part and was never built for avr-none in CI                                                              | Unconditional like the ESP crates                   |
| Feature name = the dependency it enables; ferriswheel gains `smart-leds-trait`, keeps `smart-leds-compat` as an alias | AVR `smart-leds-trait` and ESP `smart-leds` already follow the Cargo idiom; the alias avoids a break                                                          | One literal name everywhere                         |
| WS2812 pulse timings and `WS2812_RESET_US` live in `bunting`; T0H becomes 300 ns on both drivers                      | esp-idf-hal truncates 350 ns to 3 ticks, so ESP-IDF already emits 300 ns; 300 ns fits every datasheet window, 400 ns is above the WS2812B-V5 maximum (380 ns) | Reset only; everything at 400 ns                    |
| ESP-IDF `new()` uses `SOC_RMT_MEM_WORDS_PER_CHANNEL` instead of 48                                                    | ESP-IDF v5.3.3 rejects a block below the chip's size, so `new()` cannot succeed on ESP32 and S2 today                                                         | Keep 48 and document                                |
| No `build-example-*` aliases; `just build-example <example>` infers the crate                                         | `flash` and `run` already infer it from the `hal_`/`idf_` prefix; closes the roadmap alias-coverage item                                                      | Rename aliases to `build-example-{hal\|idf}-{chip}` |

## Constraints

- The pure crates and the AVR crate must keep building on the pinned `nightly-2025-04-27` (rust-version 1.88).
- `bunting`, `ferriswheel` and `pennant` stay `no_std` and allocation-free.
- Timing changes to an ESP driver invalidate earlier hardware sign-offs for that driver.
- No new dependencies.

## Open Questions

All eight were settled on 2026-11-03; the answers are the last eight Decisions rows and [ADR 011](../adr/011-cross-crate-driver-api-policy.md).

- [x] Should the ESP-IDF driver return a typed `Error` instead of `anyhow::Error`, matching esp-hal? Yes.
- [x] Should the AVR drivers gain `set_pixel` / `set_pixels_slice`, or keep `write` as their only send method? Rename, with `write` deprecated.
- [x] Should the AVR drivers implement `pennant::StatusLed`? Yes, behind `pennant`.
- [x] Should `SmartLedsWrite` stay opt-in on AVR, while both ESP crates always implement it? Yes.
- [x] Which single name should the smart-leds features use (`smart-leds-trait`, `smart-leds-compat`, `smart-leds`)? The dependency's name.
- [x] Should WS2812 pulse timings and `WS2812_RESET_US` move to `bunting`, given esp-hal uses 400 ns T0H and ESP-IDF 350 ns? Yes; ESP-IDF in fact emits 300 ns, and both drivers move to 300 ns.
- [x] Should ESP-IDF `new()` pick the RMT block size per chip, given `idf_esp32_rainbow` runs on classic ESP32 with 48 symbols? Yes; the premise was wrong, `new()` fails there today and the example was never flashed.
- [x] Should the justfile chip shortcuts follow one driver-naming pattern (today `-esp32` is IDF, `-c3`/`-c6` are HAL)? No shortcuts; `build-example` infers the crate.

## State

- [x] Design approved (open questions settled 2026-11-03)
- [x] Sweep the 19 remaining esp-hal examples to `from_channel_creator` (compile-verified on all chips; not yet flashed)
- [x] Implement `core::error::Error` for `EffectError`, `PulseEffectError`, `SpiEncodeError`, `SpiError` and `BitBangError`
- [x] Drop the `.map_err(anyhow!(…))` workarounds in the IDF examples once the error impls land
- [x] Remove the deprecated `WS2812RMT` alias (removal was promised for 0.7.0)
- [x] Extract one RGB-to-GRB-bytes helper for the three copies in the ESP-IDF driver (`bunting::rgb_to_grb_bytes`)
- [x] Change `pennant::PulseEffect::update` to take `RGB8` (slipped from 0.7.0); it takes `impl Into<RGB8>`, so tuple callers downstream keep compiling
- [x] `just build-example <example>` infers the crate; the seven `build-example-*` aliases go, which covers `idf_c3_pulse`
- [x] Flash spot checks at `a5f6b72`; ESP32 and S3 stay compile-verified only
  - [x] `hal_c3_pulse_async` on ESP32-C3 rev v0.3, GPIO4: boots, first LED pulses blue as designed (`AsyncStatusLed` drives one LED); ELF SHA-256 `f9fe0744…` unchanged by the later doc fix
  - [x] `hal_c6_fire` on ESP32-C6 rev v0.1, GPIO18: boots, flame renders correctly; ELF SHA-256 `7c7d94b3…`
  - [x] `hal_c6_pulse_async` on ESP32-C6 rev v0.1, GPIO18: boots, first LED pulses blue as designed; ELF SHA-256 `f4db6164…`
  - [x] `hal_c6_rainbow_comet_async` on ESP32-C6 rev v0.1, GPIO18 (extra): comet renders cleanly, frames logged without errors; ELF SHA-256 `905e9cf9…`
- [x] Fix the `hal_c3_pulse_async` module doc, which promised a pulse on the whole 12-LED ring
- [x] Same fix for `hal_c6_pulse_async` (ELF unchanged) and `hal_esp32_pulse_async` (image changed via LLVM symbol hashes; never flashed, so no sign-off lost)
- [x] Point `docs/embassy-task-spawning-patterns.md` at `from_channel_creator` (stale pre-1.1 `configure_tx` snippets)
- [x] Resolve the open questions above
- [x] ESP-IDF: typed `Error`, `anyhow` moved to dev-dependencies, block size from `SOC_RMT_MEM_WORDS_PER_CHANNEL`
- [x] `bunting`: WS2812 pulse and reset constants with a tick helper; both ESP drivers consume them (T0H 300 ns)
- [x] AVR: `set_pixel` / `set_pixels_slice`, deprecated `write`, `pennant` feature with `StatusLed` impls, `smart-leds-trait` in the avr-none checks; `Ws2812Spi::set_pixels_slice` now rejects a buffer below `spi_buffer_size` instead of truncating the reset tail
- [x] `ferriswheel`: `smart-leds-trait` feature with `smart-leds-compat` as alias; justfile and CI call sites moved
- [x] Hardware re-run after the T0H change (binaries built from the final tree, 2026-11-03); the classic ESP32 esp-hal pulses are still open below
  - [ ] `hal_esp32_pulse` (ELF `29d5b7f3…`) and `hal_esp32_pulse_async` (ELF `48aa38d9…`) on the classic ESP32, GPIO4: built from the final tree, not yet run; `just run <example>` with the strip on D4
  - [x] AVR `spi_rainbow` (SPI prerendered, `set_pixels_slice`) on a genuine Arduino Nano (FT232R, `/dev/cu.usbserial-AQ01L7M0`), D11: flashed 2026-11-03, ELF SHA-256 `1693c0bb…`; strip renders white-ish, the ADR 007 limitation of that backend on this strip, unchanged by the rename
  - [x] AVR `avr-nano-rainbow` (bit-bang `set_pixels_slice`, the recommended backend) on the same Nano, D11: flashed 2026-11-03, ELF SHA-256 `80f89fdb…`; clean turning rainbow on the 12-LED strip, confirmed visually; the AVR rename is hardware-verified on the recommended backend
  - [x] `hal_c6_fire` on ESP32-C6 rev v0.1, GPIO18: flashed 2026-11-03 14:24 UTC, ELF SHA-256 `26b9f05d…`; flame renders cleanly at 300 ns T0H, confirmed visually
  - [x] `hal_c3_pulse_async` on ESP32-C3 rev v0.3, GPIO4 (USB-UART port): flashed 2026-11-03 14:37 UTC, ELF SHA-256 `ce3078cb…`; first LED pulses blue cleanly at 300 ns T0H, confirmed visually
  - [x] `idf_c3_pulse` on ESP32-C3 rev v0.3, GPIO4 (USB-UART port): flashed 2026-11-03 14:38 UTC, ELF SHA-256 `766ff7c0…`; serial log shows `app_main()` entered with no error return or panic in 20 s, so `Ws2812Rmt::new()` succeeded with the chip-sized block; all 12 LEDs pulse cleanly, confirmed visually (waveform unchanged)
  - [x] `hal_s3_pulse` on ESP32-S3 rev v0.2 (CrowPanel 1.28", 5 onboard LEDs, GPIO48, native USB): flashed 2026-11-03 17:24 UTC, ELF SHA-256 `527e2894…`; all five LEDs pulse blue cleanly at 300 ns T0H, confirmed visually (first Xtensa sign-off on the new timing)
  - [x] `hal_s3_pulse_async` on ESP32-S3 rev v0.2, GPIO48: flashed 2026-11-03 17:24 UTC, ELF SHA-256 `76780bc1…`; first LED pulses blue, confirmed visually; LEDs two to five stayed static blue from the previous firmware's last frame, because the async pulse examples never send a clearing frame (`set_pixel` leaves the rest of the chain latched, as documented); not a timing fault
  - [x] `idf_s3_rainbow` on ESP32-S3 rev v0.2, GPIO48: flashed 2026-11-03 17:26 UTC, ELF SHA-256 `48ee9cc3…`; rainbow clear and smooth on all five LEDs, confirmed visually (typed error path and chip-sized block on a 48-word Xtensa chip)
  - [x] `hal_s3_pulse_async` with the clearing frame on ESP32-S3 rev v0.2, GPIO48: flashed 2026-11-03 17:29 UTC after `idf_s3_rainbow`, ELF SHA-256 `e5367a3f…`; first LED pulses blue and the other four went dark, confirmed visually
  - [x] `idf_esp32_rainbow` on classic ESP32-WROOM-32 rev v3.1, GPIO4 (USB-UART `/dev/cu.usbserial-0001`): flashed 2026-11-03 17:44 UTC by the user, ELF SHA-256 `f65cafac…`; rainbow runs on the 12-LED strip, confirmed visually; first ever run of this example, and the proof that `new()` sizes the block to the chip's 64 words
- [x] `just verify`, `just test`, the AVR checks, `check-hal-c3`, `check-hal-async`, `check-hal-xtensa`, `check-idf`, `clippy-idf` and `just deny` pass on the final tree; CHANGELOG `[Unreleased]` updated

## Session Log

- 2026-11-03 — Feature doc created from the cross-platform streamlining investigation
- 2026-11-03 — Mechanical batch done: error impls, example sweep, `map_err` cleanup, alias removal; flash spot checks pending
- 2026-11-03 — Review of `a5f6b72`: no blocker; C3 module doc fixed (binary unchanged), C6 spot checks waived
- 2026-11-03 — `hal_c6_fire` flashed at `a5f6b72` on ESP32-C6: clean; `hal_c6_pulse_async` flashed too: clean; all planned spot checks done, plus `hal_c6_rainbow_comet_async`
- 2026-11-03 — Whole-ring wording fixed in the C6 and ESP32 async pulse docs
- 2026-11-03 — AVR bit-bang rainbow (`80f89fdb…`) confirmed on the Nano; the SPI build shows the ADR 007 white-ish output as documented
- 2026-11-03 — AVR runs on a genuine Nano: ravedude does not auto-detect the FTDI port, so `RAVEDUDE_PORT=/dev/cu.usbserial-AQ01L7M0` is needed; macOS creates the node only after its accessory prompt is accepted
- 2026-11-03 — `idf_esp32_rainbow` (`f65cafac…`) ran for the first time on a classic ESP32 rev v3.1 and renders the rainbow: the 64-word block path of the block-size fix is now hardware-verified
- 2026-11-03 — Clearing frame confirmed on the S3 (`e5367a3f…`): after the rainbow, only the first LED pulses and the other four are dark
- 2026-11-03 — The four `hal_*_pulse_async` examples gained a clearing frame at start; this changes those example binaries only (the library is untouched), so the earlier async pulse hashes `ce3078cb…` (C3), `f4db6164…`/`f9fe0744…` (C6/C3 at a5f6b72) and `76780bc1…` (S3) describe the pre-clear examples; the S3 one is re-flashed below
- 2026-11-03 — S3 CrowPanel runs confirmed visually: `hal_s3_pulse` (`527e2894…`), `hal_s3_pulse_async` (`76780bc1…`) and `idf_s3_rainbow` (`48ee9cc3…`); the async pulse examples leave stale colours on the other LEDs because they never send a clearing frame
- 2026-11-03 — All three hardware runs confirmed visually: `hal_c6_fire` (`26b9f05d…`) on the C6, `hal_c3_pulse_async` (`ce3078cb…`) and `idf_c3_pulse` (`766ff7c0…`) on the C3; the feature is complete pending commit
- 2026-11-03 — Review pass (code-reviewer, rust-reviewer, justfile-reviewer, dependency-manager): esp-hal tick constants now all derive from `RMT_TICK_NS`, the frame-sweep floor follows the 1.1 µs zero code, `Error` docs corrected, SPI empty slice is a no-op; `hal_c6_fire` flashed on the C6 at `26b9f05d…`
- 2026-11-03 — Implementation of the accepted decisions landed uncommitted in five groups (bunting + ESP drivers, AVR, ferriswheel, pennant, tooling + docs); verification and hardware re-run pending
- 2026-11-03 — Eight open questions settled from code inspection, datasheet research and the downstream repos; ADR 011 drafted. Two findings changed the premises: ESP-IDF v5.3.3 rejects a 48-symbol block on ESP32 and S2 (`rmt_tx.c:247`), and esp-idf-hal 0.47 truncates 350 ns to 3 ticks at 10 MHz
