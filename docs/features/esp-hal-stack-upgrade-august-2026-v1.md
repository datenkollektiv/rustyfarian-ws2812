# esp-hal patch bump — August 2026 (v1)

**Status:** **Validated** (2026-08-12) — compile-verified and hardware-validated on ESP32-C6
**Branch:** `august-2026-release-wave`
**Cycle reference:** `audit/2026-08-12-monthly-audit.md`

All five required checks pass on an ESP32-C6, including the GPIO8 onboard-LED regression guard, plus
`hal_c3_pulse` on an ESP32-C3 — which closes a hardware-validation gap open since 2026-04-29.
Only the ESP32-WROOM-32 (Xtensa) remains compile-verified rather than hardware-validated; board unavailable.

## Summary

A patch-level bump of `esp-hal` and the `embuild` build-dependency.
This is **not** a coordinated release wave — every companion crate is unchanged — so the full `maintenance-plan.md` runbook applies in reduced form.

| Crate     | Before | After      | Pin style                      | Scope                                    |
|:----------|:-------|:-----------|:-------------------------------|:-----------------------------------------|
| `esp-hal` | 1.1.0  | **1.1.2**  | `=` exact, workspace           | bare-metal driver + all `hal_*` examples |
| `embuild` | 0.33.1 | **0.33.3** | `=` exact, IDF crate build-dep | ESP-IDF build only, no runtime footprint |

Unchanged and deliberately not touched: `esp-rtos` 0.3.0, `esp-println` 0.17.0, `esp-bootloader-esp-idf` 0.5.0, `embassy-time` 0.5.1, `embassy-executor` 0.10.0, `embassy-sync` 0.8.0.
No Embassy realignment was required (runbook step 4 was a no-op).

## Why this is low risk

- Upstream policy: **breaking changes never ship in a patch release.** The `=1.1.0` pin exists to guard against *minor* drift in the `unstable` RMT/GPIO APIs; a patch bump is not what it was defending against.
- All three companion crates are unmoved, so there is no cross-crate API realignment.
- `Cargo.lock` shows no change to `riscv` (0.15.0), `esp-riscv-rt` (0.14.0), `esp-sync` (0.2.1), or any `embassy-*` version.

## What changed upstream

**1.1.1 (2026-05-07)** — 12 fixes. The one that touches our peripheral:

> *RMT pulse length now supports maximum values on both phases.*

**Relevance assessment:** our driver emits `T0H=4, T0L=8, T1H=7, T1L=6` ticks at 100 ns/tick. These sit at the very bottom of the encodable range, so the max-value fix cannot change our output. It is called out here because it lands in the exact peripheral path this driver depends on, which is why the hardware checks below are not optional.

Also in 1.1.1: RSA interrupt handling, ESP32 ADC2 attenuation, several UART fixes, LEDC divisor off-by-one, MCPWM, SPI DMA cleanup. Removal: the ESP32 Hall-effect sensor API (unused here).

**1.1.2 (2026-08-05)** — 6 fixes: `OneShotTimer::into_blocking()` return type, ESP32 SHA power-down and accelerator locking, RSA interrupt disable ordering, an SPI import, and SPI register-block access via `ptr()` for nightly compatibility. **None touch RMT, GPIO, or anything this driver uses.**

## Compile verification — complete

All run on 2026-08-12 against the bumped graph.

| Check                                                        | Result                                            |
|:-------------------------------------------------------------|:--------------------------------------------------|
| `just check-hal` (`riscv32imac-unknown-none-elf`)            | **PASS**                                          |
| `just clippy-hal` (`-D warnings`)                            | **PASS**                                          |
| `just check-idf` (esp toolchain; validates `embuild` 0.33.3) | **PASS**                                          |
| `just verify` (fmt, deny, check, clippy, tests)              | **PASS** — 515 tests, 0 failed                    |
| `just audit`                                                 | **PASS** — 278 crates, 1 allowed warning          |
| `just deny`                                                  | **PASS** — advisories, bans, licenses, sources ok |

Example builds (runbook step 6 — one per chip family and per feature dimension):

| Example                  | Dimension                                  | Result   |
|:-------------------------|:-------------------------------------------|:---------|
| `hal_c6_pulse`           | blocking RMT, C6                           | **PASS** |
| `hal_c6_pulse_async`     | async RMT, C6                              | **PASS** |
| `hal_c6_multitask_async` | Embassy spawn + sync (most breakage-prone) | **PASS** |
| `hal_c6_smart_leds`      | `smart-leds-trait` integration             | **PASS** |
| `hal_c3_pulse`           | C3                                         | **PASS** |
| `hal_esp32_pulse`        | Xtensa WROOM-32                            | **PASS** |

**Advisory re-check:** `cargo tree -i paste` confirms `paste 1.0.15` is *still* a direct dependency of `esp-hal 1.1.2` as well as transitive via `riscv 0.15.0`. RUSTSEC-2024-0436 therefore remains suppressed; `deny.toml` updated to cite 1.1.2.

---

## Hardware validation — complete

Compile checks cannot detect RMT timing regressions, colour-order changes, or frame tearing — only a board can. **All required checks passed on 2026-08-12**; the criteria and procedure below are retained as the reusable runbook for the next esp-hal bump.

### Pass criteria (all five must hold for every run)

Per `maintenance-plan.md` § Hardware tests:

1. **Correct pattern and colour order** — the expected animation renders with no channel swap (a GRB/RGB mismatch shows as red↔green inversion).
2. **No flicker, random flashes, or frame tearing** across a continuous **60-second** run.
3. **Smooth and repeatable** brightness ramps and animation across **3 consecutive runs**.
4. **Clean serial output** — no panic, watchdog reset, backtrace, or repeated error logs.
5. **Board stability** — no unexpected reboot or USB disconnect for the full run; a rerun produces the same visual result.

> The examples embed a printing `#[panic_handler]`, so any panic surfaces on the serial monitor rather than silently hanging. Watch for it.

### Required — ESP32-C6

Wiring: WS2812B ring, **12 LEDs**, data on **GPIO18**, 300–500 Ω series resistor on the data line, common ground, 3V3 → VCC.

- [x] **1. `just run hal_c6_pulse`** — blocking RMT baseline.
      *Primary regression check for the 1.1.1 RMT change.* Expect a smooth single-colour pulse across all 12 LEDs.
- [x] **2. `just run hal_c6_multitask_async`** — Embassy multitask render + button.
      Additionally, needs a **momentary button on GPIO9** (active-low, internal pull-up — the BOOT button on most C6 dev boards). Verify the button changes the animation and that the render task keeps running while input is handled.
- [x] **3. `just run hal_c6_pulse_async`** — async RMT path.
      Confirms `into_async()` and the `Channel<'d, Async, Tx>` transmit path are unaffected.
- [x] **4. `just run hal_c6_smart_leds`** — `SmartLedsWrite` integration.
      Confirms the trait adapter still drives the same output as the native path.

### Required if boards are available

- [~] **5. `just run hal_c3_pulse`** — ESP32-C3, data on **GPIO4**, 12 LEDs.
      Deferred since 2026-04-29 for want of a board; this bump is a good moment to close it.
- [~] **6. `just run hal_esp32_pulse`** — ESP32-WROOM-32 (Xtensa), data on **GPIO4**, 12 LEDs.
      The only runtime exercise of the Xtensa target; compile-only coverage otherwise.

### Lore regression check — GPIO8 onboard LED (C6)

The historic GPIO8 transmit hang (`txn.wait()` blocking forever on first call) was resolved by the 1.1.0 upgrade with **no workaround in our driver** — so a future upstream RMT regression would resurface it silently. 1.1.1 *does* modify RMT, which makes this worth one explicit check.

This is now covered by a permanent example, `hal_c6_onboard_pulse` (added 2026-08-12), so no pin edit is required:

- [x] **7. GPIO8 onboard SK68XXMINI check** (ESP32-C6-DevKitC-1)
  1. `just run hal_c6_onboard_pulse` with **no external ring attached**.
  2. Expect the onboard LED to pulse blue. **A hang inside the first `txn.wait()` means the 1.1.0 fix regressed — stop and do not merge.**

  The example prints `starting RMT transmit loop on GPIO8` immediately *before* the first transmit, so the serial log pins a hang to `set_pixels_slice`. Note that serial alone cannot fully close this check: a silent hang and normal operation produce identical output, because the render loop prints nothing. **The pulsing LED is the deciding evidence.**

### IDF driver examples — not required, but validated anyway

`embuild` is a *build-time* dependency with no runtime footprint, so `just check-idf` passing was deemed sufficient and no IDF hardware run was required for this bump.

It happened regardless: **`just run idf_c3_rainbow` confirmed working on an ESP32-C3 on 2026-08-12.** That upgrades the `embuild 0.33.3` evidence from compile-only to runtime, and separately confirms the `esp-idf-sys` two-bootloader fix works end to end on a second architecture.

**This does not cover check 5.** `idf_c3_rainbow` exercises `rustyfarian-esp-idf-ws2812` (ESP-IDF RMT API), which does not depend on `esp-hal` in any way. The `esp-hal 1.1.2` bump is therefore still unexercised on C3 — only `hal_c3_pulse` (bare-metal, `rustyfarian-esp-hal-ws2812`) can close that.

### Sign-off

| # | Check                    | Board        | Result      | Date       | Notes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
|:--|:-------------------------|:-------------|:------------|:-----------|:-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 1 | `hal_c6_pulse`           | C6           | **PASS**    | 2026-08-12 | Flashed OK (100,160 B) to esp32c6 rev v0.0. 20 s serial capture: single clean boot via IDF v5.3.3 bootloader, app loaded at 0x10000, no panic / watchdog / boot loop; `efuse block revision v0.1` accepted. Pulse animation confirmed rendering on hardware by maintainer — **no RMT regression from the 1.1.1 pulse-length change.**                                                                                                                                                                        |
| 2 | `hal_c6_multitask_async` | C6 + button  | **PASS**    | 2026-08-12 | Confirmed working on hardware by maintainer. Exercises the Embassy task-spawn path and multi-task render + button architecture — the dimension most likely to break on an esp-hal bump.                                                                                                                                                                                                                                                                                                                      |
| 3 | `hal_c6_pulse_async`     | C6           | **PASS**    | 2026-08-12 | Confirmed working on hardware by maintainer. Covers `into_async()` and the `Channel<'d, Async, Tx>` transmit path.                                                                                                                                                                                                                                                                                                                                                                                           |
| 4 | `hal_c6_smart_leds`      | C6           | **PASS**    | 2026-08-12 | Confirmed working on hardware by maintainer. Covers the `SmartLedsWrite` trait adapter.                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 5 | `hal_c3_pulse`           | C3           | **PASS**    | 2026-08-12 | Confirmed working on hardware by maintainer (GPIO4, 12 LEDs). **Closes a gap open since 2026-04-29** — the bare-metal `esp-hal` path had never been hardware-exercised on C3. Distinct from `idf_c3_rainbow`, which uses the ESP-IDF driver and does not touch `esp-hal`.                                                                                                                                                                                                                                    |
| 6 | `hal_esp32_pulse`        | WROOM-32     | **N/A**     | 2026-08-12 | Board unavailable — see above. Compile-verified only (Xtensa target builds clean).                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 7 | GPIO8 onboard LED        | C6 DevKitC-1 | **PASS**    | 2026-08-12 | Now covered by the permanent `hal_c6_onboard_pulse` example (no pin edit needed). Flashed OK (100,032 B); 22 s capture shows the pre-transmit marker printed, a single boot, no panic / watchdog / reboot. Onboard LED confirmed pulsing on hardware by maintainer — **the GPIO8 transmit hang has not regressed under esp-hal 1.1.2.** Re-validated after the `.ok()` → `.expect()` error-handling change (code review follow-up), confirming the guard's fail-loud path does not fire on the healthy path. |

**Definition of done** — *exceeded on 2026-08-12*: checks 1–4 and 7 passed on an ESP32-C6, and check 5 passed on an ESP32-C3 that became available during the cycle. Only check 6 (WROOM-32) is recorded as "board unavailable" rather than skipped. `CHANGELOG.md` and this document's status were updated accordingly.

Check 5 is the notable one: the bare-metal `esp-hal` path on C3 had been deferred for want of a board since 2026-04-29 and is now hardware-validated. Coverage after this cycle: **C6 fully validated across blocking, async, Embassy multi-task, `smart-leds`, and the GPIO8 onboard path; C3 validated on both the bare-metal and ESP-IDF drivers; Xtensa WROOM-32 compile-only.**

## Open questions

1. ~~**Should a GPIO8 example be added permanently?**~~ **Resolved 2026-08-12** — `hal_c6_onboard_pulse` added. The onboard-LED path now has standing regression coverage and check 7 is a single command rather than a hand-edited pin.
2. **Should `check-hal-xtensa` join the routine gate?** `hal_esp32_pulse` compiles, but the Xtensa target is only exercised on demand.
