---
gate: none
desk-work: available
---

# Feature: ESP32-S3 support in `rustyfarian-esp-hal-ws2812` v1

Requested from the downstream `rustyfarian-knob` project (upstream request U1 of its esp-hal parity quest), filed 2026-10-13.
The knob's CrowPanel 1.28" ESP32-S3 drives a 5-LED WS2812 ring on GPIO48 and could not use the esp-hal driver without an `esp32s3` feature.

## Decisions
| Decision                                                       | Reason                                                       | Rejected Alternative                                 |
|:---------------------------------------------------------------|:-------------------------------------------------------------|:-----------------------------------------------------|
| `esp32s3` feature forwards to hal, bootloader, rtos?, println? | Mirrors `esp32`; library has no chip `cfg`                   | Chip `cfg` inside `src/lib.rs`                       |
| Examples `hal_s3_pulse` + `hal_s3_pulse_async`                 | Mirror `hal_esp32_pulse*`; blocking + async paths            | One example only                                     |
| Data pin GPIO48                                                | Drives the CrowPanel knob's 5-LED ring, the validation board | Neutral pin (GPIO4)                                  |
| `NUM_LEDS = 5` in S3 examples                                  | Matches the CrowPanel ring                                   | 12, as in the ESP32 examples                         |
| Scripts treat Xtensa as `esp32` or `s3`                        | Both need `cargo +esp` and `-Zbuild-std=core`                | Separate S3 branch                                   |
| CI `buildtargets: esp32,esp32s3`, new cache key                | Old key would restore an esp32-only toolchain                | Second S3 job                                        |
| No version bump here                                           | Release (0.7.1 vs 0.8.0) is a separate decision              | Bump in this change                                  |
| S3 bootloader from new `idf_s3_rainbow`                        | Same proven IDF v5.3.3 path as C3/C6/ESP32                   | espflash bundled v5.5 (unproven, lore says it fails) |
| IDF docs: S3 uses 48-symbol RMT blocks                         | Registry metadata; knob ships `Ws2812Rmt::new()` on S3       | Old text grouping S3 with ESP32/S2 (64)              |

## Constraints
- Stack stays at esp-hal =1.2.2, esp-rtos =0.4.0, esp-bootloader-esp-idf =0.6.0, esp-println =0.18.0.
- MSRV 1.95; the local `esp` toolchain is 1.95.0.0, CI pins 1.97.0.0.
- Existing `esp32`, `esp32c3` and `esp32c6` builds stay unchanged.
- Every build goes through `just`; no raw `cargo`.
- On S3, RMT TX is channels 0–3 with 48 words each, as on the C6 (verified in `esp-metadata-generated` 0.5.3).
- The S3 RMT clock defaults to APB 80 MHz, so `Rate::from_mhz(80)` with `RMT_CLK_DIV = 8` is valid (verified in esp-hal 1.2.2).
- `idf_s3_rainbow` uses `gpio48`, so it only compiles for the S3; no recipe builds IDF examples for other targets.
- `.cargo/config.toml` is gitignored, so the new S3 target sections are local only, like the existing ones.

## Open Questions
- [x] Which bootloader does `just run hal_s3_*` flash? The IDF v5.3.3 one, built from `idf_s3_rainbow`.

## State
- [x] Design approved
- [x] Core implementation
- [x] Checks passing (`just check-hal-xtensa`, `just verify`)
- [x] Hardware-validated on the CrowPanel ESP32-S3
- [x] Documentation updated (README, CHANGELOG, version matrix, maintenance plan)

## Hardware sign-off
Run 2026-10-14 on the CrowPanel 1.28" against esp-hal 1.2.2, esp-rtos 0.4.0, esp-bootloader-esp-idf 0.6.0, esp-println 0.18.0.
All `maintenance-plan.md` criteria met: 60 s without flicker, 3 repeatable runs, clean serial, no resets.

| Example              | Observed                                            | Result   | Binary sha256 |
|:---------------------|:----------------------------------------------------|:---------|:--------------|
| `hal_s3_pulse`       | All five LEDs pulse blue                            | **PASS** | `b508fdda…`   |
| `hal_s3_pulse_async` | First LED pulses blue (`AsyncStatusLed`, by design) | **PASS** | `1626170a…`   |
| `idf_s3_rainbow`     | Rainbow spread over all five LEDs                   | **PASS** | —             |

## Session Log
- 2026-10-13 — Feature doc created from upstream request U1; upstream facts verified against registry sources.
- 2026-10-14 — Implemented; full check chain green, S3 examples link, IDF S3 bootloader built; hardware run pending.
- 2026-10-14 — Hardware-validated on the CrowPanel; async header corrected without changing the flashed binary.
