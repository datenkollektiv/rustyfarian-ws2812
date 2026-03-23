# ADR 008: Embassy as the Async Runtime

## Status

Accepted

## Context

ADR 006 decided that `rustyfarian-esp-hal-ws2812` should offer async variants of `set_pixel` and `set_pixels_slice` behind an `async` feature flag.
That ADR intentionally deferred the choice of async runtime.

An async RMT driver needs two things from its runtime:

1. **A timer** — `embassy-time` provides `Timer::after_millis().await` for inter-frame delays.
2. **An executor** — something must poll the futures.

On ESP32 bare-metal (`no_std`), several options exist:

| Runtime                            | Executor                            | Timer                                 | RMT integration                                      | Maintenance              |
|:-----------------------------------|:------------------------------------|:--------------------------------------|:-----------------------------------------------------|:-------------------------|
| `esp-rtos` 0.2 + Embassy           | Thread-mode via `#[esp_rtos::main]` | `embassy-time` (driver in `esp-rtos`) | `esp-hal` native async channel (`Rmt::into_async()`) | Active (Espressif)       |
| `embassy-executor` standalone      | Manual `Executor::run()`            | Requires custom time driver           | Same                                                 | Active (Embassy project) |
| `edge-executor`                    | Minimal, no timer                   | None built-in                         | Same                                                 | Minimal maintenance      |
| No runtime (Option A from ADR 006) | N/A                                 | N/A                                   | Blocking only                                        | N/A                      |

On ESP-IDF (`std`), the situation is different: `esp-idf-hal 0.46` has no async RMT support.
The `TxChannelDriver` is blocking-only.
If async RMT appears in a future `esp-idf-hal` release, it will likely use `esp-idf-svc`'s own executor or `tokio` — not Embassy.

### Why this matters

The runtime choice affects:

- **Which crates appear in `Cargo.toml`** — `esp-rtos`, `embassy-time`, `embassy-executor`
- **Which `#[main]` macro users write** — `#[esp_rtos::main]` vs manual executor setup
- **Which timer API examples demonstrate** — `Timer::after_millis().await`
- **Version coupling** — `esp-rtos` 0.2 pins compatible `embassy-time` and `embassy-executor` versions

### What the review queue raised

A review of Embassy's fit in the Rustyfarian ecosystem (see `archive/review-queue/`) confirmed that:

- `esp-rtos` is Espressif's official integration of Embassy for `esp-hal 1.0+`
- The pure logic crates (`ferriswheel`, `ws2812-pure`, `led-effects`) are completely unaffected by the runtime choice
- `esp-hal`'s `Rmt::into_async()` produces channels that work directly with Embassy's waker mechanism
- The `SmartLedsWriteAsync` trait and a potential `AsyncStatusLed` trait are natural follow-ons

## Decision

**Use Embassy via `esp-rtos` as the async runtime for `rustyfarian-esp-hal-ws2812`'s `async` feature.**

Specifically:

- `esp-rtos` provides the executor and `embassy-time` driver
- `embassy-time` provides `Timer`, `Instant`, `Duration`
- `embassy-executor` provides the `Spawner` type for `#[esp_rtos::main]`
- All three are workspace dependencies, version-locked together

### Boundaries

| Crate                        | Embassy dependency           | Rationale                                                                     |
|:-----------------------------|:-----------------------------|:------------------------------------------------------------------------------|
| `rustyfarian-esp-hal-ws2812` | Yes (behind `async` feature) | Wraps `esp-hal`'s async RMT channel                                           |
| `rustyfarian-esp-idf-ws2812` | No                           | `esp-idf-hal` has no async RMT; if it gains one, it will use its own executor |
| `rustyfarian-avr-ws2812`     | No                           | AVR has no async ecosystem                                                    |
| `ferriswheel`                | No — never                   | Pure computation, no I/O or runtime deps                                      |
| `ws2812-pure`                | No — never                   | Pure computation                                                              |
| `led-effects`                | No — never                   | Pure traits and logic                                                         |

The "never" entries are a hard constraint.
If a future contributor proposes adding `embassy-time` to `ferriswheel` for a convenience `run_effect` loop, that proposal should be rejected per ADR 006 (Option C was explicitly rejected) and this ADR.

### Why not a separate crate?

The review queue document suggested a `rustyfarian-embassy-ws2812` crate.
This was considered and rejected for consistency with ADR 005 and ADR 006:

- ADR 005 splits crates along the **HAL boundary** (`esp-idf-hal` vs `esp-hal`), not along the runtime boundary
- ADR 006 decided async is a **feature flag** on the existing HAL driver, not a separate crate
- `esp-hal` itself uses `Blocking`/`Async` as a type parameter on the same struct, not separate crates
- A third driver crate would triple the maintenance surface with no architectural benefit

The `async` feature flag on `rustyfarian-esp-hal-ws2812` is the right granularity.

## Consequences

### Positive

- **Single official runtime** — no ambiguity about which executor or timer to use with the async driver
- **Espressif-blessed** — `esp-rtos` is maintained by Espressif and tested against `esp-hal` releases
- **Version coherence** — `esp-rtos` 0.2 declares compatible versions of `embassy-time` and `embassy-executor`; workspace deps inherit this
- **Pure crates stay pure** — the boundary is absolute: Embassy never enters `ferriswheel`, `ws2812-pure`, or `led-effects`

### Negative

- **Embassy version coupling** — upgrading `esp-rtos` may require coordinated bumps of `embassy-time` and `embassy-executor`
- **Ecosystem churn** — Embassy is pre-1.0; breaking changes are possible (mitigated by pinning via `esp-rtos`)
- **ESP-IDF divergence** — the IDF driver cannot share the same async pattern; if `esp-idf-hal` gains async RMT, it will likely use a different executor

### Follow-on items (tracked in roadmap)

- **`SmartLedsWriteAsync`** for `Ws2812Rmt<'d, Async, N>` — natural integration with the `smart-leds` ecosystem's async trait
- **`AsyncStatusLed` trait** in `led-effects` — allows async drivers to implement status feedback without blocking
- **Monitor `esp-idf-hal`** for async RMT support — would enable async for the IDF driver under a different runtime
