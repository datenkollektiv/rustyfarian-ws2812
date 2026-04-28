# Feature: esp-hal Stack Upgrade — April 2026 Release Wave v1

Upgrade every crate in the bare-metal `esp-hal` stack used by `rustyfarian-esp-hal-ws2812` from the `esp-hal 1.0.0` baseline (released October 2025, currently pinned via `Cargo.lock` only) to the April 2026 release wave that landed on crates.io between 2026-04-16 and 2026-04-24.

## Version Table

| Crate                    | Current (`Cargo.lock`)  | Target  | Released   |
|:-------------------------|:------------------------|:--------|:-----------|
| `esp-hal`                | 1.0.0                   | 1.1.0   | 2026-04-24 |
| `esp-rtos`               | 0.2.0                   | 0.3.0   | 2026-04-16 |
| `esp-bootloader-esp-idf` | 0.4.0                   | 0.5.0   | 2026-04-16 |
| `esp-println`            | 0.16.1                  | 0.17.0  | 2026-04-16 |

The 2026-04-16 timestamps reflect a coordinated monorepo release wave; `esp-hal 1.1.0` followed eight days later.
The pre-1.0 crates each got a minor bump (0.x → 0.x+1), which in their semver convention typically signals breaking API changes.

## Pinning Note

Before this feature, workspace `Cargo.toml` declared `esp-hal = { version = "1.0.0" }` and similar shorthand for the others.
Cargo treats these as `^1.0.0` / `^0.2.0` etc., so the *constraint* allowed compatible future releases even though local builds were held back by `Cargo.lock`.

This repository intentionally ignores `Cargo.lock` because it is a library workspace, so the manifest now exact-pins the coordinated April 2026 stack with `=...` constraints:

| Crate                    | Exact pin |
|:-------------------------|:----------|
| `esp-hal`                | `=1.1.0`  |
| `esp-rtos`               | `=0.3.0`  |
| `esp-bootloader-esp-idf` | `=0.5.0`  |
| `esp-println`            | `=0.17.0` |
| `embassy-time`           | `=0.5.1`  |
| `embassy-executor`       | `=0.10.0` |
| `embassy-sync`           | `=0.8.0`  |

Exact pins are intentional because this driver uses `esp-hal/unstable` RMT and GPIO matrix APIs, and companion crates are released as coordinated waves.
Future bumps should land via the maintenance workflow rather than transparent fresh-resolution drift.

## Decisions

| Decision                                                                                                                          | Reason                                                                                                                                                                                                        | Rejected Alternative                                                                                                                                                    |
|:----------------------------------------------------------------------------------------------------------------------------------|:--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Coordinated workspace upgrade of the consumed crates in one feature                                                               | The pre-1.0 crates ship as a monorepo wave; mixing 0.2 + 0.3 across `esp-rtos` and friends has historically produced version-resolution conflicts                                                              | Bump only `esp-hal` first to test the GPIO8 hang in isolation — rejected because `esp-hal 1.1.0` likely tightens trait bounds that ripple into `esp-rtos`              |
| Bump in foundation order: `esp-hal` → `esp-rtos` → `esp-println` → `esp-bootloader-esp-idf`                                       | `esp-hal` is the trait/PAC root; the others depend on it transitively. Fixing breakage from the bottom up keeps the error surface small per step                                                              | Bump everything at once with one `cargo update` — rejected because cascading errors are harder to attribute                                                             |
| Re-validate on real hardware before declaring the upgrade complete                                                                | The GPIO8 hang is hardware-observable only; host tests do not exercise the RMT TX path                                                                                                                        | Trust `cargo check` + clippy alone — rejected because the existing GPIO8 bug already passed those                                                                       |
| Treat this as a prerequisite for the GPIO8 debug task (`docs/project-lore.md` "esp-hal Bare-Metal Driver" → GPIO8 hang entry)     | RMT changes between `esp-hal 1.0.0` and `1.1.0` may already have fixed it; doing the deep dive on 1.0.0 risks debugging a stale codebase                                                                      | Investigate the hang on 1.0.0 first — rejected; documented in the roadmap under the GPIO8 debug task                                                                    |
| Exact-pin the upgraded stack in `Cargo.toml`                                                                                     | `Cargo.lock` is ignored for this library workspace, and the HAL surface used here is explicitly unstable and hardware-sensitive                                                                               | Keep caret constraints and rely on maintainers to notice fresh-resolution drift                                                                                         |

## Constraints

- `rustyfarian-esp-idf-ws2812` must continue to build and run unchanged — it uses the `esp-idf-hal` stack, not `esp-hal`, so this upgrade should not touch it. Any incidental change there is out of scope.
- All three target chips must continue to work: ESP32-C6 (`riscv32imac-unknown-none-elf`), ESP32-C3 (`riscv32imc-unknown-none-elf`), and ESP32 / WROOM-1 (`xtensa-esp32-none-elf`).
- `--release` profile remains required (`docs/project-lore.md` "esp-hal Bare-Metal Driver" → `--release` entry).
- Continue to flash with the IDF v5.3.3 bootloader workaround (`docs/project-lore.md` "espflash") unless `esp-bootloader-esp-idf 0.5` changes that requirement.
- `MAX_LEDS` and the `Effect` trait contract are unchanged by this work; if a constraint surfaces, it belongs in a separate feature.

## Migration Steps

1. **Bump constraints in workspace `Cargo.toml`** to exact pins: `esp-hal = "=1.1.0"`, `esp-rtos = "=0.3.0"`, `esp-bootloader-esp-idf = "=0.5.0"`, `esp-println = "=0.17.0"`, plus the aligned Embassy crates.
2. **Run `just update`** (or `cargo update -p <each>`) to refresh `Cargo.lock`.
3. **Resolve breakage in foundation order**: `esp-hal` → `esp-rtos` → `esp-println` → `esp-bootloader-esp-idf`. After each, run `just check-hal` and `just clippy-hal` before moving on.
4. **Refresh the version-matrix doc**: `docs/esp-hal-1.0.0-version-matrix.md` — either rename to `…-1.1.0-…` or replace contents with the new resolved set.
5. **Re-test all `hal_*` examples on hardware**: `hal_c3_pulse`, `hal_c3_pulse_async`, `hal_c6_pulse`, `hal_c6_pulse_async`, `hal_c6_breathe_color`, `hal_c6_cylon`, `hal_c6_fire`, `hal_c6_knight_rider`, `hal_c6_meteor`, `hal_c6_multitask_async`, `hal_c6_rainbow_comet`, `hal_c6_rainbow_comet_async`, `hal_c6_smart_leds`, `hal_c6_twinkle`, `hal_esp32_pulse`, `hal_esp32_pulse_async`. Use `just run <name>` for each chip variant available.
6. **Re-test the GPIO8 hang specifically** by temporarily editing `hal_c6_pulse.rs` to `peripherals.GPIO8` + `NUM_LEDS = 1` (the same delta as April 2026's session). Record the outcome in `docs/project-lore.md` regardless of whether the hang persists.
7. **Update `CHANGELOG.md` under `## [Unreleased]`** with one bullet per crate bump and a note about whether the GPIO8 hang is now fixed.
8. **Run the full `just verify` (or `just pre-commit`)** to confirm fmt, deny, check, lint, and tests all pass on the upgraded stack.

## Open Questions

- [x] Does `esp-hal 1.1.0` already fix the GPIO8 RMT blocking transmit hang on ESP32-C6-DevKitC-1? **Yes — confirmed by hardware retest 2026-04-29.** `hal_c6_pulse` with `peripherals.GPIO8` + `NUM_LEDS = 1` pulses the onboard SK68XXMINI LED correctly on the upgraded stack; no workaround needed in our driver.
- [x] Are there breaking changes in `esp-rtos 0.3` to `#[esp_rtos::main]`, `esp_rtos::start()`, or the embassy executor surface that affect `hal_c6_multitask_async` or the planned Chromatic Clash demo? **Yes, handled.** `embassy-executor 0.10` changed the task-spawn shape: `Spawner::spawn` now returns `()`, while `#[embassy_executor::task]` functions return `Result<SpawnToken, SpawnError>`. The working pattern is `spawner.spawn(task().unwrap())`.
- [x] Does `esp-bootloader-esp-idf 0.5` still produce an `esp_app_desc_t` accepted by the IDF v5.3.3 bootloader, or does it require a newer IDF? **Accepted in the checked HAL builds.** Keep the existing IDF v5.3.3 bootloader workaround unless a hardware flashing test proves it obsolete.
- [x] Does `embassy-sync` still resolve to `0.7` under `esp-rtos 0.3`, or does the workspace pin need to move to `0.8`? **Moved to `0.8.0`.** `esp-sync 0.2.1` still pulls older `embassy-sync` versions internally for compatibility shims, but application code resolves through the workspace-pinned `embassy-sync 0.8.0`.
- [x] Does `esp-println 0.17` change the `jtag-serial` feature behaviour (e.g. ROM-based vs USB-Serial-JTAG peripheral) — relevant for the panicking `println!` handler kept in examples? **No compile-time break found.** Representative C6 async examples with `esp-println` still compile.
- [ ] Does the upgraded stack compile for the ESP32 / WROOM-32 Xtensa bare-metal examples in this local environment? **Not verified here.** `cargo check --target xtensa-esp32-none-elf` could not run because the Xtensa target/core was not installed locally; this remains a toolchain-gated follow-up.

## Outcome

- Upgraded the workspace bare-metal stack to the April 2026 wave.
- Migrated all `hal_*` RMT setup call sites to the `esp-hal 1.1.0` `configure_tx(&config).unwrap().with_pin(pin)` pattern.
- Migrated `hal_c6_multitask_async` to the `embassy-executor 0.10` spawn pattern.
- Exact-pinned the coordinated stack in `Cargo.toml` because this library workspace ignores `Cargo.lock`.
- Updated `CHANGELOG.md`, `docs/esp-hal-version-matrix.md`, `docs/project-lore.md`, and `docs/ROADMAP.md`.
- Confirmed by hardware retest that the ESP32-C6 GPIO8 RMT blocking transmit hang is fixed on `esp-hal 1.1.0`.

## Validation Evidence

- `just check` — passed.
- `just check-hal` — passed.
- `cargo check -p rustyfarian-esp-hal-ws2812 --target riscv32imac-unknown-none-elf --no-default-features --features esp32c6,unstable,led-effects` — passed.
- `cargo check -p rustyfarian-esp-hal-ws2812 --examples --target riscv32imac-unknown-none-elf --features rt,async,smart-leds,esp-println` — passed.
- `cargo check -p rustyfarian-esp-hal-ws2812 --example hal_c3_pulse --target riscv32imc-unknown-none-elf --no-default-features --features esp32c3,unstable,rt,led-effects` — passed.
- `cargo check -p rustyfarian-esp-hal-ws2812 --example hal_c3_pulse_async --target riscv32imc-unknown-none-elf --no-default-features --features esp32c3,unstable,rt,led-effects,async` — passed.
- `just test` — passed.
- `just fmt-check` — passed.

Residual risk: the ESP32 / WROOM-32 Xtensa examples still need a local check with the Xtensa bare-metal toolchain installed.

## State

- [x] Design approved
- [x] Core implementation
- [x] Tests passing
- [x] Documentation updated

## Session Log

- 2026-04-29 — Feature doc created from roadmap content; supersedes the inline upgrade entry in `docs/ROADMAP.md`. Triggered by the GPIO8 RMT hang investigation, which surfaced that `esp-hal 1.0.0` is no longer the latest stable release.
- 2026-04-29 — Upgrade implemented and reviewed. Follow-up tightened the workspace from caret constraints to exact pins.
