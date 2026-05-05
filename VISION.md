# Project Vision

## North Star

Provide reusable, sans-io WS2812 LED crates for embedded Rust developers —
pure logic first, hardware wrappers thin, everything testable without hardware.

## Long-Term Goals

- **Animation vocabulary on demand.**
  Cover the animation and effect primitives that downstream users actually need.
  The set is not predefined — it grows in response to real requests.
  "Done" is the state where users consistently find what they need without forking.
  Rings remain the centre of gravity for `ferriswheel`; matrix/grid types
  exist as supporting building blocks, not as a strategic direction.

- **Complete no_std / embassy support.**
  Keep the `rustyfarian-esp-hal-ws2812` driver feature-current with the
  `esp-hal` ecosystem so downstream projects can adopt `no_std` / embassy
  without losing WS2812 support.

- **Ecosystem currency across supported MCU families.**
  Timely adoption of new ESP32 chip variants, HAL updates, toolchain changes,
  and AVR toolchain shifts.
  The embedded Rust ecosystem moves fast; the crates must keep pace on
  every supported target.

- **Preserve the sans-io discipline.**
  All pure logic stays in `no_std`-compatible, hardware-free crates.
  Hardware wrappers remain thin.
  No new crate or feature should break this separation.

## Target Beneficiaries

Embedded Rust developers building WS2812-based LED projects on ESP32 or AVR —
the two first-class supported MCU families — who want testable, composable
building blocks rather than monolithic driver crates.

Primary today: the maintainer's own downstream project(s).
Secondary: any embedded developer who discovers and adopts the crates.

## Supported Platforms

- **ESP32 (RISC-V and Xtensa)** via `rustyfarian-esp-idf-ws2812` (std, ESP-IDF RMT)
  and `rustyfarian-esp-hal-ws2812` (no_std, esp-hal RMT, blocking and async).
- **AVR (ATmega328P and compatible)** via `rustyfarian-avr-ws2812`
  (no_std, SPI prerendered or cycle-counted bit-bang per ADR 007).

Additional MCU families are not pursued proactively, but remain open if a
genuine use case arises and the sans-io discipline can be preserved.

## Non-Goals

- **Application code, binaries, or end-user products.**
  This workspace is library-only.
  Demo projects exploring networking, multi-device coordination, or finished
  products — e.g. ESP-NOW LED demos — live in separate workspaces and consume
  these crates as dependencies.
- **Predefined exhaustive animation catalogues.**
  Effects are added on demand, not speculatively.
- **Matrix-first animation vocabulary.**
  Grid/matrix types may exist as building blocks but are not a strategic
  focus.
  Ring animations remain the primary domain for `ferriswheel`.
- **Random stray functionality** that does not serve the embedded WS2812
  use case.
- **Proactive expansion to additional MCU families** beyond ESP32 and AVR.

## Success Signals

- Downstream users find the animation or effect they need without forking
  or copying code.
- `rustyfarian-esp-hal-ws2812` stays current with the `esp-hal` release
  cadence and is in use by at least one `no_std` / embassy project.
- New ESP32 chip variants, AVR toolchain shifts, and HAL releases are
  adopted within a reasonable window of their release.
- All pure logic remains fully unit-testable on a laptop without ESP or
  AVR toolchains, and without hardware.

## Open Questions

- If third-party users arrive (issues, forks), how should the demand-driven
  animation model scale?
  The current informal loop works while the primary consumer is the
  maintainer.

## Vision History

- 2026-02-25 — Initial vision created during the first Vision Validator session.
  Identified esp-hal driver completion as the most strategically important
  missing roadmap item.
- 2026-03-01 — Vision review confirmed goals are sound; no_std/embassy driver
  completion was missing from the roadmap despite being the stated top
  priority.
  Refocused near-term order: `NoLed` stub first (simple, unblocks
  downstream), then `esp-hal` driver, then ecosystem integration
  (`SmartLedsWrite`, `smart-leds` color types) behind it.
- 2026-05-05 — Promoted AVR to a first-class supported MCU family alongside
  ESP32 (ADR 007 validated, driver in production).
  Confirmed rings remain the centre of gravity for `ferriswheel`; the new
  `grid` module is a building block, not a strategic shift toward matrices.
  Tightened non-goals to exclude in-workspace demo and networking projects
  (e.g. ESP-NOW demos belong in a separate workspace that consumes these
  crates).
  Promoted crates.io publication from an open question to a near-term
  roadmap action, motivated by the maintainer's own consumption story
  (cleaner than git dependencies).
