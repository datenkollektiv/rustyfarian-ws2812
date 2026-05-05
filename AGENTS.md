# AGENTS.md

> Use this file as the fast-path operating guide for AI coding agents.
> Prefer repository truth over assumptions — check the files referenced below.

## Project Overview

`rustyfarian-ws2812` is a Cargo workspace of reusable WS2812 / NeoPixel LED crates for embedded Rust.
The design philosophy is **sans-io**: pure animation logic lives in `no_std` crates with no hardware dependency, while three thin hardware wrappers (ESP-IDF, esp-hal, AVR) provide the actual driver glue.
Target hardware: ESP32-C3 / C6 / WROOM-32 and ATmega328P. MSRV is `1.88`.

## Architecture

Six workspace members under `crates/`, plus one standalone example:

| Crate | Role | Target |
|:------|:-----|:-------|
| `bunting` | color conversion, bit encoding, grid layout | `no_std` |
| `ferriswheel` | 14 ring animations + `Effect` trait | `no_std` |
| `pennant` | status-LED adapter + `StatusLed` / `AsyncStatusLed` traits | `no_std` |
| `rustyfarian-esp-idf-ws2812` | ESP-IDF RMT driver | `std` (ESP-IDF) |
| `rustyfarian-esp-hal-ws2812` | esp-hal RMT driver, blocking + async | `no_std` (bare-metal) |
| `rustyfarian-avr-ws2812` | AVR SPI + bit-bang driver | `no_std` |
| `examples/avr-nano-rainbow/` | Arduino Nano demo (standalone, not in workspace) | AVR |

The `Effect` trait in `crates/ferriswheel/src/effect.rs` is the contract every animation implements (`update()`, `current()`, `reset()`).
ADRs in `docs/adr/` document the load-bearing decisions; `docs/project-lore.md` records hard-won debugging insights.

## Development Workflow

The `justfile` is the canonical interface — it handles target overrides, since `.cargo/config.toml` defaults the workspace to `riscv32imac-esp-espidf`.

| Command | Purpose |
|:--------|:--------|
| `just verify` | Primary CI gate: fmt-check + deny + check + clippy + test on host target |
| `just pre-commit` | fmt + check + clippy + test (auto-fixes formatting) |
| `just check-hal` | Compile-check the bare-metal esp-hal driver against `riscv32imac-unknown-none-elf` |
| `just check-idf` | Compile-check the ESP-IDF driver (requires `espup` toolchain) |
| `just check-avr` / `just check-avr-target` | AVR driver on host / on real `avr-none` (nightly + `avr-gcc`) |
| `just test` | Unit and doc tests on host target |
| `just run <example>` | Flash an example to a connected ESP32 board |
| `just build-example <crate> <name>` | Build an example without flashing |

For ad-hoc cargo invocations on platform-independent crates, always pass `--target` (e.g. `--target aarch64-apple-darwin`) — the workspace default points at ESP-IDF.

## Key Conventions

**Exact-pinned esp-hal stack.** `Cargo.toml` pins `esp-hal`, `esp-rtos`, `esp-bootloader-esp-idf`, `esp-println`, `embassy-executor`, `embassy-sync`, and `embassy-time` with `=` constraints. Bumps are coordinated waves — never relax to caret. See `docs/esp-hal-version-matrix.md`.

**Example naming.** `{driver}_{chip}_{effect}`: `hal_c6_pulse`, `idf_c3_rainbow`, `hal_esp32_pulse_async`. Async variants take an `_async` suffix and require `--features async`.

**Effect builder pattern.** Constructors return `Result<Self, EffectError>`; `with_*` methods chain. Inherent methods + `impl Effect` delegation avoids recursion. See any file under `crates/ferriswheel/src/` for the pattern (e.g. `pulse.rs`).

**`Cargo.toml` writes vs reads.** The workspace ignores `Cargo.lock` (it's a library workspace). Library bumps must use exact pins in `[workspace.dependencies]` to prevent caret drift.

**Documentation style.** One sentence per line. Use `sh` fences (not `bash`/`shell`). Never put comments inside code snippets — explanatory text goes above the snippet. ADRs follow Michael Nygard format under `docs/adr/NNN-short-description.md`.

**Changelog and ADRs.** Behaviour changes go under `## [Unreleased]` in `CHANGELOG.md` (Keep a Changelog format). Architectural decisions get a new ADR before the code lands.

## Important Files

- `justfile` — every standard task; read first when unsure how to build/test something
- `Cargo.toml` (root) — workspace dependency pins and the rationale comments
- `.cargo/config.toml` — default target, per-target rustflags / runners / linkers
- `crates/ferriswheel/src/effect.rs` — `Effect` trait, `EffectError`, `MAX_LEDS`
- `docs/project-lore.md` — debugging gotchas not derivable from the source
- `docs/ROADMAP.md` and `docs/adr/` — what's planned and why decisions look the way they do
- `CHANGELOG.md` — release history; latest is `[0.5.0] 2026-05-05`
