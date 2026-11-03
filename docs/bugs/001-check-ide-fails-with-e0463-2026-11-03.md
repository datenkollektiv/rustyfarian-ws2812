---
id: 001
title: just check-ide fails with E0463 on the bare-metal RISC-V target
captured-on: 2026-11-03
doc-version: 2
status: open-observation
kind: observation
---

# Bug 001: `just check-ide` fails with E0463 on the bare-metal RISC-V target

## Symptom
`just check-ide` fails with `error[E0463]: can't find crate for std` and "cannot resolve a prelude import" while compiling `rustyfarian-esp-idf-ws2812` (lib).

## Suspected Cause
The recipe checks the ESP-IDF crate for `riscv32imac-unknown-none-elf`, where no `std` exists.
Since 2026-05-27 the crate compiles as an empty crate on non-IDF targets via `#![cfg(target_os = "espidf")]` ([separate build environments v1](../features/archive/separate-build-environments-v1.md)).
An empty crate still injects the `std` prelude, which cannot resolve on `riscv32imac-unknown-none-elf`.
Likely fix: add `#![cfg_attr(not(target_os = "espidf"), no_std)]`, or drop the recipe if its premise no longer holds.

## Linked Artefact
- `justfile` recipe `check-ide` (`cargo check -p rustyfarian-esp-idf-ws2812 --target {{ hal_target }}`)
- `crates/rustyfarian-esp-idf-ws2812/src/lib.rs`
- First noted during [driver interchangeability v1](../features/archive/driver-interchangeability-v1.md), on a clean HEAD export

## Reproduction Confidence
high — reproduced on `a530c4b` on 2026-11-03.

## Session Log
- 2026-11-03 — Observation captured via /bug dialog; split out of driver interchangeability v1, reproduced on `a530c4b`
- 2026-11-03 — Suspected cause narrowed: the `#![cfg(target_os = "espidf")]` empty-crate gating still pulls in the `std` prelude
