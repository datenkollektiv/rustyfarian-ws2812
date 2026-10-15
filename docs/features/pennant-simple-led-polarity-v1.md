---
gate: none
desk-work: available
---

# Feature: `pennant::SimpleLed` honours `Polarity` v1

Requested from the downstream `rustbox-spookhouse` project for its ESP-IDF prop on the ESP32-C3 SuperMini, filed 2026-10-15 and needed by 2026-10-20.
The SuperMini's blue LED on GPIO8 is active-low, so `SimpleLed` lights when it should be dark and the consumer keeps a local wrapper instead of using `StatusLed`.

## Decisions
| Decision                                                             | Reason                                                       | Rejected Alternative                       |
|:---------------------------------------------------------------------|:-------------------------------------------------------------|:-------------------------------------------|
| `polarity: Polarity` field on `SimpleLed`, default `ActiveHigh`      | Default behaviour unchanged; additive                        | Separate `ActiveLowLed` type               |
| `#[must_use] fn with_polarity(mut self, Polarity) -> Self` builder   | Mirrors `RgbGpioLed::with_polarity`                          | Extra constructor argument (breaking)      |
| `SimpleLed::with_threshold(pin, threshold)` constructor stays as is  | Changing it to a builder would break callers                 | Align it with the `RgbGpioLed` builder now |
| `Polarity` and `set_channel` move to a private `polarity.rs` module  | Shared by three adapters; `pennant::Polarity` path unchanged | `pub(crate) use` from `rgb_gpio_led.rs`    |
| `set_color` calls `set_channel(&mut self.pin, on, self.polarity)`    | One drive-level decision for all adapters                    | Inline match in `SimpleLed`                |
| New tests join `simple_led_tests` in `lib.rs`, reusing its `MockPin` | That is where the existing `SimpleLed` tests live            | New test module in `simple_led.rs`         |
| No version bump here; 0.7.1 is a separate release decision           | Same policy as the S3 and async-split features               | Bump `pennant` to 0.7.1 in this change     |

## Constraints
- Existing `SimpleLed` tests pass unchanged; `just test-pennant-hal`, `just clippy-pennant-hal` and `just verify` stay green.
- `SimpleLed` keeps its single `P: OutputPin` bound, so esp-idf-hal 0.47 `PinDriver` and esp-hal 1.2 `Output` compile as before.
- `just check-hal` and `just check-hal-c3` still pass, since the esp-hal driver enables `pennant`.
- Docs in `simple_led.rs` gain an active-low ESP-IDF example (SuperMini GPIO8) next to the existing one.
- `crates/pennant/README.md` and the root README mention polarity for `SimpleLed`; CHANGELOG gets an `Added` entry under `[Unreleased]`.
- Write-on-change caching stays out of scope.

## Open Questions
- [ ] Publish `pennant` 0.7.1 before 2026-10-20 so the consumer can drop its Git pin?

## State
- [x] Design approved
- [x] Core implementation
- [x] Tests passing (`just test-pennant-hal`, `just clippy-pennant-hal`, `just check-hal`, `just check-hal-c3`, `just verify`)
- [x] Documentation updated (`simple_led.rs` docs, pennant README, root README, CHANGELOG)

## Session Log
- 2026-10-15 — Feature doc created via /feature dialog
- 2026-10-15 — Implemented test-first via workflow (test-automator, rust-engineer, documentation-engineer, code-reviewer, rust-reviewer); five new tests; not yet committed
