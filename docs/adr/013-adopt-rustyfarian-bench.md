# ADR 013: Adopt rustyfarian-bench for On-Device Checks

## Status

Proposed.
Records a decision already in use: the bench was introduced in `ee1a78f` on 2026-11-08 and prepared the v0.9.0 hardware pass.
Matches `rustyfarian-network` ADR 024 and the sibling repos that run their checks through the same runner.

## Context

The drivers are validated on hardware by flashing an example with `just run <example>`, watching the LED ring and reading the serial log by eye.
The v0.9.0 [final hardware pass](../../release-plan.md) spans four ESP chips on two tiers plus the Arduino Nano, and most of its cells are the same pulse and rainbow on a different board.
The rustyfarian family has `rustyfarian-bench`, a guided runner (Python + uv): a consumer describes boards and checks as TOML under `bench/`, and the runner builds, finds the board, flashes, resets, captures serial and judges each step.
Most examples print nothing, so a verdict on the ring needs an operator's answer, while a crash or reboot can be judged from the serial log alone.
The C3 fixtures `hal_c3_frame_sweep_async`, `hal_c3_rainbow_comet_async` and `hal_c3_colors` (with its twin `idf_c3_colors`) do log over serial, so their checks can be judged automatically.

## Decision

1. Single-board on-device checks are written as `rustyfarian-bench` scenarios under `bench/<board>/`, for the boards `s3` (CrowPanel 1.28" HMI), `c3-mini1` (ESP32-C3-DevKitM-1), `c6`, `esp32` (ESP32-WROOM-32) and `avr` (Arduino Nano).
2. Every board runs the same two checks first, `pulse` and `rainbow`; on the ESP boards `pulse` covers esp-hal (blocking and async) and `rainbow` covers ESP-IDF.
3. The C3 board adds `colors`, `comet` and `frame-sweep`.
4. `bench/bench.toml` calls the existing `just build-example` and `just flash` recipes, with `ESPFLASH_PORT` set to the detected port, and the AVR board calls `just build-avr-bin` and `just flash-avr-bin`; no bench-only build or flash recipes are added.
5. `just bench`, `just bench-list` and `just bench-detect` match the sibling repos: they run the runner release pinned in the `justfile` through `uvx` (v0.5.0), and `RUSTYFARIAN_BENCH` points them at a local checkout instead.
6. Every check splits into an unattended part (logs, soaks, crash watches) and an attended part (a 60 s watch, then one question); a full pass needs both.
7. Run logs and `results.jsonl` stay git-ignored, because results record board MACs and this repository is public.
8. `spi_rainbow`, the logic-analyser captures and the radio-coexistence benches stay manual or downstream until they get their own step design.

## Consequences

- The pulse and rainbow cells become rerunnable and comparable across tiers and chips.
- An automatic verdict covers what serial output can prove; an explicit operator answer covers what it cannot (colours, smoothness).
- Bench runs depend on `uv` and on GitHub access for the pinned runner (or a local checkout); builds, `just verify` and CI are unaffected.
- The Nano cannot be found by espflash, so it is found by its USB identity, which needs runner v0.4.0 or later.
- Scenario regexes couple to example log lines; changing a log line means updating its scenario.
- A bench result counts as a hardware sign-off only for the binary that was run; a later edit to the driver needs a new pass.
