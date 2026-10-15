---
gate: none
desk-work: available
---

# Feature: Split the `async` feature from the example runtime in `rustyfarian-esp-hal-ws2812` v1

Requested from the downstream `rustyfarian-peripherals` project (same esp-hal wave), filed 2026-10-14.
A consumer enabling `async` for `Ws2812Rmt<'_, Async, N>` is forced to take `esp-rtos` and the `embassy-*` crates, although `src/lib.rs` uses only `esp_hal::Async`.

## Decisions
| Decision                                                                               | Reason                                                             | Rejected Alternative                         |
|:---------------------------------------------------------------------------------------|:-------------------------------------------------------------------|:---------------------------------------------|
| `async = ["esp-hal/unstable"]`, dependency-free                                        | Gates only the two `#[cfg(feature = "async")]` impl blocks         | Compile async always, `async` as no-op alias |
| New `embassy` feature pulls esp-rtos, embassy-time/-executor/-sync, `esp-rtos/embassy` | Only the `*_async` examples use the runtime                        | Keep runtime inside `async`                  |
| `embassy` implies `async`                                                              | Examples need both; one flag in `required-features`                | Examples list both features                  |
| Chip features keep `esp-rtos?/<chip>` weak entries                                     | Still needed when `embassy` is on                                  | Move chip wiring into `embassy`              |
| `*_async` examples switch `required-features` to `embassy`                             | They are the only runtime users                                    | Leave on `async` and add deps back           |
| Build scripts map `*_async` examples to `embassy`                                      | `scripts/build-example.sh:65`, `run-example.sh:68` set the feature | New `*_embassy` example naming               |
| Ship in the next 0.x minor with a CHANGELOG note                                       | Removing deps from `async` is semver-visible                       | Patch release                                |
| No version bump in this change                                                         | Release is a separate decision, as for the S3 feature              | Bump here                                    |

## Constraints
- `cargo tree --features <chip>,async` must show no `esp-rtos`, `embassy-time` or `embassy-executor`.
- `embassy-sync`, `embassy-embedded-hal`, `embassy-futures` and `embassy-hal-internal` stay in every graph; `esp-hal` 1.2.2 pulls them itself.
- All six `*_async` examples build and run unchanged via `just build-example` / `just run`.
- `just check-hal`, `just check-hal-c3`, `just check-hal-xtensa` and `just verify` stay green.
- docs.rs metadata stays on the default feature set; no runtime crate is needed for docs.
- ADR 008's Boundaries table and the crate README feature table must name the new feature.
- Precedent: `rustyfarian-peripherals` async rotary `Encoder` uses the same `async` / `embassy` split.

## Open Questions
- [ ] Should the GitHub cross-target workflows build one `embassy` example to guard the new feature wiring? No CI job builds any `*_async` example today; one `hal_c6_pulse_async` check in the RISC-V job would cover it.

## State
- [x] Design approved
- [x] Core implementation
- [x] Checks passing (`just check-hal`, `just check-hal-c3`, `just check-hal-xtensa`, all six `*_async` example builds, `just verify`)
- [x] Documentation updated (crate README, root README, AGENTS.md, lib.rs docs, ADR 008, CHANGELOG)

## Session Log
- 2026-10-15 — Feature doc created via /feature dialog
- 2026-10-15 — Implemented via workflow (build-engineer, documentation-engineer, code-reviewer, dependency-manager); `cargo deny` clean; not yet committed
