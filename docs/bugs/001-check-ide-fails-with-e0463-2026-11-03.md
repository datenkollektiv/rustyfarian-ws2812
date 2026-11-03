---
id: 001
title: just check-ide fails with E0463 on the bare-metal RISC-V target
captured-on: 2026-11-03
doc-version: 1
status: open-observation
kind: observation
---

# Bug 001: `just check-ide` fails with E0463 on the bare-metal RISC-V target

## Symptom
`just check-ide` fails with `error[E0463]: can't find crate for std` and "cannot resolve a prelude import" while compiling `rustyfarian-esp-idf-ws2812` (lib).

## Suspected Cause
The recipe checks the ESP-IDF crate for `riscv32imac-unknown-none-elf`, where no `std` exists.
The crate has no `no_std` attribute for non-`espidf` targets, so its implicit `std` prelude cannot resolve there.
Either the crate needs `#![cfg_attr(not(target_os = "espidf"), no_std)]`, or the recipe's premise no longer holds and it should go.

## Linked Artefact
- `justfile` recipe `check-ide` (`cargo check -p rustyfarian-esp-idf-ws2812 --target {{ hal_target }}`)
- `crates/rustyfarian-esp-idf-ws2812/src/lib.rs`
- First noted during [driver interchangeability v1](../features/archive/driver-interchangeability-v1.md), on a clean HEAD export

## Reproduction Confidence
high — reproduced on `a530c4b` on 2026-11-03.

## Session Log
- 2026-11-03 — Observation captured via /bug dialog; split out of driver interchangeability v1, reproduced on `a530c4b`
