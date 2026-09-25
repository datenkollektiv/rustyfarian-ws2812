# esp-hal stack upgrade — September 2026 (v1)

**Status:** Validated (2026-09-24) — compile-verified on ESP32-C6, ESP32-C3 and Xtensa ESP32 (2026-09-21), and hardware-validated on ESP32-C6 and ESP32-C3; ESP32-WROOM-32 (Xtensa) remains compile-verified only
**Branch:** `september-2026-maintenance`
**Cycle reference:** `audit/2026-09-21-quarterly-plan.md` (change 4), `audit/2026-09-21-upstream-verification.md`

## Summary

A coordinated bump of the four exact-pinned `esp-*` crates, plus the MSRV raise they require.
Unlike April, this is not a single-day upstream wave: three companions shipped together on 2026-08-26 and `esp-hal 1.2.0` followed on 2026-09-02, with 1.2.1 and 1.2.2 as independent patch releases.
It is still one upgrade for us, because `esp-rtos 0.4` requires `esp-hal 1.2`.

| Crate                    | Before | After      | Published  | Pin style            |
|:-------------------------|:-------|:-----------|:-----------|:---------------------|
| `esp-hal`                | 1.1.2  | **1.2.2**  | 2026-09-18 | `=` exact, workspace |
| `esp-rtos`               | 0.3.0  | **0.4.0**  | 2026-08-26 | `=` exact, workspace |
| `esp-bootloader-esp-idf` | 0.5.0  | **0.6.0**  | 2026-08-26 | `=` exact, workspace |
| `esp-println`            | 0.17.0 | **0.18.0** | 2026-08-26 | `=` exact, workspace |

Unchanged and deliberately not touched: `embassy-time 0.5.1`, `embassy-executor 0.10.0`, `embassy-sync 0.8.0`.
`esp-rtos 0.4.0` still requires `embassy-executor 0.10` and `embassy-sync 0.8`, and does not depend on `embassy-time` at all (it uses `embassy-time-driver 0.2` and `embassy-time-queue-utils 0.3`).
Runbook step 4 (Embassy realignment) is therefore a no-op, established on a scratch graph before any workspace edit.

## Why we take it

The substantive payload is in `esp-rtos 0.4.0`, the runtime under every async example:

- "fixed a potential crash on RISC-V devices (#5641)"
- "deleting a task no longer allows reallocating its stack memory while the stack may be in use (#6032)"
- "main task stack sizes are correctly tracked (#6027)" and "the idle tasks now perform stack overflow checking (#6027)"

`esp-hal 1.2.x` itself changes nothing in the RMT or GPIO surface this driver uses.
Its RMT entries are ESP32-P4 / ESP32-S31 chip support only; the "pulse length on both phases" fix shipped in 1.1.1 and was already on board.

## MSRV decision

Every crate in the wave declares `rust-version = "1.95.0"`.
Decision (2026-09-21): the maintainer chose a workspace-wide 1.95 with `rustyfarian-avr-ws2812` as the single exception.
Outcome: that exact shape is **not implementable**, and the applied shape is the nearest faithful reading — the whole AVR build path stays at 1.88, the two ESP driver crates go to 1.95.

The first attempt set the workspace to 1.95 and the AVR crate to 1.88.
`just check-avr-target` then failed with `rustc 1.88.0-nightly is not supported by the following package: bunting@0.6.0 requires rustc 1.95`.
Cargo enforces `rust-version` on every package in the graph, path dependencies included, so the pure crates the AVR driver consumes (`bunting` directly; `ferriswheel` via the AVR example binaries) are bound by the same nightly.
The nightly cannot move on its own: upstream `avr-hal` main and `avr-hal-template` pin the same date, `avr-none` is Tier 3 behind the still-unstable `-Z build-std`, and AVR `asm!` needs `asm_experimental_arch`.
See `docs/project-lore.md` § Toolchain & Dependencies.

| File                                           | Before                          | After                                                              |
|:-----------------------------------------------|:--------------------------------|:-------------------------------------------------------------------|
| `Cargo.toml` (workspace)                       | `rust-version = "1.88"`         | unchanged, with a comment on the bound                             |
| `crates/rustyfarian-esp-hal-ws2812/Cargo.toml` | `rust-version.workspace = true` | `rust-version = "1.95"`                                            |
| `crates/rustyfarian-esp-idf-ws2812/Cargo.toml` | `rust-version.workspace = true` | `rust-version = "1.95"` (by policy; the IDF stack needs only 1.82) |
| `crates/rustyfarian-avr-ws2812/Cargo.toml`     | `rust-version.workspace = true` | unchanged                                                          |

Consequence: only the two ESP driver crates drop 1.88–1.94 support at their next crates.io release; `bunting`, `pennant`, `ferriswheel` and the AVR driver keep 1.88.
The rejected alternative was passing `--ignore-rust-version` in the AVR recipes and workflows, which would make the declared MSRV false for AVR builds.
Every toolchain in use satisfies 1.95 except the AVR nightly: local stable 1.95.0, local `esp` 1.95.0.0, CI `dtolnay/rust-toolchain@stable`, Xtensa 1.97.0.0.

## The one API break — `esp_rtos::start`

`esp-hal 1.2.0` removed `SoftwareInterruptControl`, `SoftwareInterrupt::steal` and the `SW_INTERRUPT` singleton (#6142) and added `FROM_CPU_INTRn` peripheral singletons.
`esp-rtos 0.4.0`'s entry point is now `pub fn start(timer: impl TimerSource, int0: FROM_CPU_INTR0<'static>)`, with no architecture `cfg`.
`FROM_CPU_INTR0` exists for `esp32`, `esp32c3` and `esp32c6` in `esp-metadata-generated 0.5.3`.

Migration, one call site per async example:

```rust
esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);
```

| Example                         | Change                                                                                 |
|:--------------------------------|:---------------------------------------------------------------------------------------|
| `hal_c3_pulse_async.rs`         | drop the `SoftwareInterruptControl` import and `sw_ints` line; pass `FROM_CPU_INTR0`   |
| `hal_c6_pulse_async.rs`         | same                                                                                   |
| `hal_c6_rainbow_comet_async.rs` | same                                                                                   |
| `hal_c6_multitask_async.rs`     | same                                                                                   |
| `hal_esp32_pulse_async.rs`      | one-arg `start` becomes two-arg; the comment explaining why Xtensa differed is deleted |

The library (`src/lib.rs`) does not touch software interrupts and needs no change.

## Graph changes

Resolved on a scratch crate before the bump and confirmed after it:

- `esp-sync` 0.2.1 → 0.3.0 (still ships the deliberate `embassy-sync` 0.6.2 / 0.7.2 / 0.8.0 shim set).
- `esp-riscv-rt` 0.14.0 → 0.15.0, `esp-hal-procmacros` → 0.23.0, `esp-config` → 0.8.0, `esp-metadata-generated` → 0.5.3.
- `riscv` stays 0.15.0.
- **New transitives:** `esp-storage 0.10.0` (via `esp-bootloader-esp-idf 0.6.0`), `esp-alloc 0.11.0`, `static_cell`, `rlsf`, `linked_list_allocator`, `allocator-api2`, `const-default`, `hybrid-array`, `somni-template`, and the PAC crates `esp32c5`, `esp32c61`, `esp32p4`, `esp32s31`.
- `paste 1.0.15` remains a **direct** dependency of `esp-hal 1.2.2` (`paste = "1.0.15"` in its published `Cargo.toml`) and transitive via `riscv 0.15.0`; RUSTSEC-2024-0436 stays suppressed and `deny.toml` is re-cited to 1.2.2.

## Compile verification

All run on 2026-09-21 against the final manifests, in this order, after the MSRV layout was corrected (see § MSRV decision).

| Check                                               | Result                                                                                 |
|:----------------------------------------------------|:---------------------------------------------------------------------------------------|
| `just check-avr-target` (proves the AVR MSRV bound) | **PASS**                                                                               |
| `just check-avr-target-bitbang`                     | **PASS**                                                                               |
| `just build-avr-example-all-bins` (`--locked`)      | **PASS**                                                                               |
| `just check-hal` (`riscv32imac-unknown-none-elf`)   | **PASS**                                                                               |
| `just check-hal-c3` (library + all C3 examples)     | **PASS**                                                                               |
| `just clippy-hal` (`-D warnings`)                   | **PASS**                                                                               |
| `just check-hal-xtensa` (`cargo +esp`, 1.95.0.0)    | **PASS** — answers open question 1: `1.95.0-nightly` satisfies `rust-version = "1.95"` |
| `just check-idf` (esp-idf-hal 0.47.0, unaffected)   | **PASS**                                                                               |
| `just verify`                                       | **PASS** — 515 tests, 0 failed, 11 binaries                                            |
| `just audit` / `just deny`                          | **PASS** — 1 allowed advisory (`paste`); advisories, bans, licences, sources ok        |

Example builds (runbook step 6), all **PASS**:

| Example                  | Dimension                                  | Result   |
|:-------------------------|:-------------------------------------------|:---------|
| `hal_c6_pulse`           | blocking RMT, C6                           | **PASS** |
| `hal_c6_pulse_async`     | async RMT, C6                              | **PASS** |
| `hal_c6_multitask_async` | Embassy spawn + sync                       | **PASS** |
| `hal_c6_smart_leds`      | `smart-leds-trait` integration             | **PASS** |
| `hal_c6_onboard_pulse`   | GPIO8 regression guard (build only)        | **PASS** |
| `hal_c3_pulse`           | C3                                         | **PASS** |
| `hal_c3_pulse_async`     | C3 async                                   | **PASS** |
| `hal_esp32_pulse`        | Xtensa WROOM-32                            | **PASS** |
| `hal_esp32_pulse_async`  | Xtensa async — the call shape that changed | **PASS** |

Dependency-manager sign-off on the new graph: **APPROVE** — every added crate is MIT/Apache-2.0; `esp-alloc` is reachable only via the `async` feature; the `esp32c5`/`c61`/`p4`/`s31` PACs are resolved by Cargo but feature-gated out of compilation; all application-reachable `embassy-sync` paths resolve to 0.8.0; no added crate declares an MSRV above 1.95.

## Hardware validation — PASSED (C6 and C3)

Run on 2026-09-24 against the pinned stack this document describes — `esp-hal 1.2.2`, `esp-rtos 0.4.0`, `esp-bootloader-esp-idf 0.6.0`, `esp-println 0.18.0` — as released in `v0.7.0`.
The `esp-rtos` scheduler changed underneath every async example, so this was not a formality.

Pass criteria are those in `maintenance-plan.md` § Hardware tests (60 s run, no flicker or tearing, 3 repeatable runs, clean serial, board stable); every check below met them in full.

- [x] **1. `just run hal_c6_pulse`** — blocking RMT baseline.
- [x] **2. `just run hal_c6_multitask_async`** — **primary**: Embassy spawn plus the migrated `start`; needs the GPIO9 button.
- [x] **3. `just run hal_c6_pulse_async`** — async RMT path.
- [x] **4. `just run hal_c6_smart_leds`** — `SmartLedsWrite` adapter.
- [x] **5. `just run hal_c6_onboard_pulse`** — GPIO8 regression guard; a hang in the first `txn.wait()` means stop and do not merge.
- [x] **6. `just run hal_c3_pulse`** and **`just run hal_c3_pulse_async`** — C3, GPIO4.
- [ ] **7. `hal_esp32_pulse` / `hal_esp32_pulse_async`** — WROOM-32; N/A unless a board appears.

A sign-off is valid only for the binary that ran.
It is invalidated by a change to `rustyfarian-esp-hal-ws2812`, to the example it names, or to any pinned `esp-*` version in the table above — not by unrelated commits, rebases or squashes.

### Sign-off

| # | Check                       | Board    | Result   | Date       | Notes                                         |
|:--|:----------------------------|:---------|:---------|:-----------|:----------------------------------------------|
| 1 | `hal_c6_pulse`              | C6       | **PASS** | 2026-09-24 | blocking RMT baseline                         |
| 2 | `hal_c6_multitask_async`    | C6       | **PASS** | 2026-09-24 | primary check — Embassy spawn, GPIO9 button   |
| 3 | `hal_c6_pulse_async`        | C6       | **PASS** | 2026-09-24 | async RMT path under `esp-rtos 0.4.0`         |
| 4 | `hal_c6_smart_leds`         | C6       | **PASS** | 2026-09-24 | `SmartLedsWrite` adapter                      |
| 5 | `hal_c6_onboard_pulse`      | C6       | **PASS** | 2026-09-24 | GPIO8 regression guard — no `txn.wait()` hang |
| 6 | `hal_c3_pulse` (+ async)    | C3       | **PASS** | 2026-09-24 | GPIO4, both blocking and async                |
| 7 | `hal_esp32_pulse` (+ async) | WROOM-32 | N/A      | —          | board unavailable since 2026-04-29            |

## Open questions

1. ~~**Does `cargo` accept the `esp` toolchain's `1.95.0-nightly` against `rust-version = "1.95"`?**~~ **Resolved 2026-09-21 — yes.** `just check-hal-xtensa` passed with the esp-hal driver crate declaring 1.95.
2. ~~**Do any of the new transitives reach the library's runtime graph?**~~ **Resolved 2026-09-21.** `esp-storage`, `sdio`, `static_cell`, `somni-template`, `embassy-net-driver` and the extra PACs are resolved through `esp-hal 1.2.2` / `esp-bootloader-esp-idf 0.6.0` with default features; `esp-alloc` and its allocator chain only through `async`; the off-target PACs never compile.

## Session Log

- 2026-09-21 — Planned in the quarterly cycle; upstream claims verified against tagged sources; MSRV decision taken (workspace 1.95, AVR 1.88); doc drafted ahead of the bump.
- 2026-09-21 — Bump applied; five async examples migrated to `FROM_CPU_INTR0`; first MSRV layout failed `just check-avr-target` on `bunting@0.6.0 requires rustc 1.95`; corrected to workspace 1.88 with the two ESP driver crates at 1.95; full chain re-run on the final manifests, all PASS; hardware validation not run (no boards).
- 2026-09-24 — Hardware validation run on ESP32-C6 and ESP32-C3: checks 1–6 all PASS to the full `maintenance-plan.md` criteria, including the GPIO8 onboard regression guard and the migrated `esp_rtos::start` under Embassy multitasking. Check 7 (WROOM-32) stays N/A — no board. The upgrade is no longer compile-verified only.
