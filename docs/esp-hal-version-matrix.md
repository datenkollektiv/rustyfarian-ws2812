# esp-hal Companion Crate Version Matrix

## Current State — `esp-hal 1.1.2` (April 2026 release wave + August patch)

The workspace pins the coordinated companion crates from the wave released on 2026-04-16
(with `esp-hal 1.1.0` itself on 2026-04-24), tracked by
[`docs/features/archive/esp-hal-stack-upgrade-april-2026-v1.md`](features/archive/esp-hal-stack-upgrade-april-2026-v1.md)
and applied during the 2026-04-29 quarterly maintenance pass.

`esp-hal` was subsequently bumped **1.1.0 → 1.1.2** on 2026-08-12 (patch-only; every companion
crate unchanged, so this was an isolated bump rather than a new wave). See
[`docs/features/esp-hal-stack-upgrade-august-2026-v1.md`](features/esp-hal-stack-upgrade-august-2026-v1.md).

| Crate                    | Current    | Notes                                                                                                                                                                                     |
|:-------------------------|:-----------|:------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `esp-hal`                | **1.1.2**  | `configure_tx` signature change (from 1.1.0): takes `&TxChannelConfig`, pin attaches via `.with_pin(...)` chain. 1.1.1 added an RMT max-pulse-length fix; 1.1.2 is SHA/RSA/SPI/Timer only |
| `esp-rtos`               | **0.3.0**  | Still bundles Embassy time driver; pulls `embassy-sync 0.8`, `embassy-executor 0.10` transitively                                                                                         |
| `esp-bootloader-esp-idf` | **0.5.0**  | Still produces an `esp_app_desc_t` accepted by IDF v5.3.3 bootloader                                                                                                                      |
| `esp-println`            | **0.17.0** |                                                                                                                                                                                           |
| `embassy-executor`       | **0.10.0** | `Spawner::spawn` now returns `()`; task functions return `Result<SpawnToken, SpawnError>` — pattern: `spawner.spawn(task().unwrap());`                                                    |
| `embassy-sync`           | **0.8.0**  | Direct workspace dep aligned with `esp-rtos 0.3` transitive resolution                                                                                                                    |
| `embassy-time`           | **0.5.1**  | Patch-level update                                                                                                                                                                        |

`esp-sync 0.2.1` (an internal monorepo crate) deliberately depends on three `embassy-sync` versions
(0.6.2 + 0.7.2 + 0.8.0) for backwards compatibility — this is intentional upstream and explains
why `Cargo.lock` resolves multiple versions. Application code resolves to a single
`embassy-sync 0.8.0` via the workspace direct dependency.

---

## Historical Reference — `esp-hal 1.0.0` (October 2025)

The remainder of this document records the exact versions of every crate in the
`esp-rs/esp-hal` monorepo that were tagged and published alongside `esp-hal 1.0.0` on
**October 30, 2025**.
Kept as a historical reference for understanding why our constraints evolved as they did.

## Summary

`esp-hal 1.0.0` is the first stable release of the Espressif Rust HAL.
The monorepo contains many companion crates that are versioned and released independently
but are co-developed and tested together.
Not all companions were tagged on October 30 — some shipped two weeks earlier with `v1.0.0-rc.1`
(October 13, 2024) and were not re-tagged for the final release.
The table below records the version that was current at the `esp-hal-v1.0.0` tag for each crate.

Key structural changes relative to pre-1.0 releases:

- `esp-wifi` was renamed to **`esp-radio`** (starting at rc.1, October 2025).
- `esp-hal-embassy` was merged into **`esp-rtos`** (starting at rc.1).
- `esp-rtos` now bundles the Embassy time driver and executor integration.
- `esp-radio-rtos-driver` is a new internal glue crate between `esp-radio` and `esp-rtos`.

## Version Matrix

| Crate                    | Version at `esp-hal-v1.0.0` tag | Tag date   | Notes                                        |
|:-------------------------|:--------------------------------|:-----------|:---------------------------------------------|
| `esp-hal`                | **1.0.0**                       | 2025-10-30 | Main HAL — first stable release              |
| `esp-radio`              | **0.17.0**                      | 2025-10-30 | Replaces `esp-wifi`; still unstable          |
| `esp-rtos`               | **0.2.0**                       | 2025-10-30 | Absorbs `esp-hal-embassy`; unstable          |
| `esp-radio-rtos-driver`  | **0.2.0**                       | 2025-10-30 | Internal glue crate                          |
| `esp-bootloader-esp-idf` | **0.4.0**                       | 2025-10-30 | App descriptor macro for IDF bootloader      |
| `esp-alloc`              | **0.9.0**                       | 2025-10-13 | Global allocator; tagged at rc.1             |
| `esp-backtrace`          | **0.18.0**                      | 2025-10-13 | Panic handler; tagged at rc.1                |
| `esp-println`            | **0.16.0**                      | 2025-10-13 | `defmt`/UART/JTAG output; tagged at rc.1     |
| `esp-storage`            | **0.8.0**                       | 2025-10-13 | NVS flash abstraction; tagged at rc.1        |
| `esp-lp-hal`             | **0.3.0**                       | 2025-10-30 | Low-power co-processor HAL                   |
| `esp-phy`                | **0.1.0**                       | 2025-10-30 | RF PHY layer (internal)                      |
| `esp-sync`               | **0.1.0**                       | 2025-10-30 | `no_std` sync primitives                     |
| `esp-config`             | **0.6.0**                       | 2025-10-30 | Build-time configuration helpers             |
| `esp-hal-procmacros`     | **0.21.0**                      | 2025-10-30 | Proc-macros used by `esp-hal`                |
| `esp-riscv-rt`           | **0.13.0**                      | 2025-10-30 | RISC-V runtime (replaces `riscv-rt` for ESP) |
| `xtensa-lx`              | **0.13.0**                      | 2025-10-30 | Xtensa low-level support                     |
| `xtensa-lx-rt`           | **0.21.0**                      | 2025-10-30 | Xtensa runtime                               |

### Crates absent from the monorepo at this tag

| Crate             | Status                              |
|:------------------|:------------------------------------|
| `esp-wifi`        | Removed — superseded by `esp-radio` |
| `esp-hal-embassy` | Removed — merged into `esp-rtos`    |

## esp-radio 0.17.0 Key Dependencies

These are the notable transitive dependencies pulled in by `esp-radio 0.17.0`,
relevant for understanding what a wireless-capable project needs.

| Dependency     | Version |
|:---------------|:--------|
| `smoltcp`      | 0.12.0  |
| `bt-hci`       | 0.6.0   |
| `heapless`     | 0.9     |
| `ieee802154`   | 0.6.1   |
| `esp-wifi-sys` | 0.8.1   |
| `esp-alloc`    | 0.9.0   |

## esp-rtos 0.2.0 Key Dependencies

| Dependency                 | Version          |
|:---------------------------|:-----------------|
| `embassy-executor`         | 0.9.0            |
| `embassy-sync`             | 0.7              |
| `embassy-time-driver`      | 0.2.1            |
| `embassy-time-queue-utils` | 0.3.0            |
| `esp-alloc`                | 0.9.0 (optional) |

## esp-hal 1.0.0 Rust Toolchain Requirements

| Requirement          | Value    |
|:---------------------|:---------|
| MSRV                 | `1.88.0` |
| Edition              | 2024     |
| `embedded-hal`       | 1.0.0    |
| `embedded-hal-async` | 1.0.0    |

## Stability Notes

`esp-hal 1.0.0` is the **only stable crate** in the monorepo at this tag.
Every companion crate (`esp-radio`, `esp-rtos`, `esp-alloc`, `esp-backtrace`, etc.)
remains pre-1.0 and uses the `unstable` feature gate where required by `esp-hal`.
The blog post for this release explicitly states that `esp-radio` is the next
stabilisation target.

Features requiring the `unstable` gate in `esp-hal 1.0.0`:

- RMT peripheral
- `esp-radio` integration
- Any API marked `#[instability::unstable]`

## Implications for `rustyfarian-esp-hal-ws2812`

The workspace currently pins `esp-hal = "=1.1.2"` with `features = ["esp32c6", "unstable"]`.
The current `esp-rtos` dependency is at `0.3.0` (released 2026-04-16).

If ESP-NOW or WiFi support were added to this workspace in the future, the compatible
version would be `esp-radio = "0.18.0"` with `features = ["esp-now", "unstable"]` (released 2026-04-16).

---

*Research date: 2026-03-27. Dates re-verified 2026-05-05 against the [crates.io versions API](https://crates.io/api/v1/crates/esp-hal/versions): `esp-hal 1.0.0` was published 2025-10-30, `1.0.0-rc.1` on 2025-10-13.*
*Sources: GitHub tag `esp-rs/esp-hal @ esp-hal-v1.0.0`, individual crate `Cargo.toml` files,
GitHub release pages, and the Espressif developer blog.*
