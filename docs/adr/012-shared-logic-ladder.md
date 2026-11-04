# ADR 012: Shared tier logic follows a three-rung ladder

## Status

Accepted (2026-11-04).
Adopts the ladder of `rustyfarian-network` ADR 023, `rustyfarian-power` ADR-004 and `rustyfarian-peripherals` ADR-010, so the rungs, rules and vocabulary match across the family.
Decisions 1 to 11 are the shared core; this repo's specifics are in Context, Instances and Retrofit.
A review on 2026-11-04 tightened the wording of decisions 1, 2 and 4 to 7; the sibling ADRs have not adopted the tightened text yet.
It refines the sans-IO split of [ADR 003](003-rename-core-to-pure.md) and [ADR 005](005-dual-hal-strategy.md) and supersedes neither.

## Context

Where two driver crates sequence the same flow, each re-implements the order of the steps, and the two copies drift.
`rustyfarian-network` ADR 022 decision 10 answers that drift with sans-IO state machines: inputs go in, actions come out, and a thin loop per tier performs them.
The sibling ADRs turn this into a three-rung ladder, so a flow gets a machine only when a generic driver or a pure function cannot cover it.

This repo differs from the siblings in four ways that change the wording, not the ideas.

- The shared logic lives in three pure crates, not one: `bunting` (encoding, timing, grid), `ferriswheel` (effects) and `pennant` (status LEDs).
- There are three tiers, and the esp-hal tier alone runs two execution models, blocking and async.
- Most shared logic is flat.
  `ferriswheel` effects are frame-stepped: `Effect::update` advances one frame and never reads a clock, so the caller paces the animation.
- The logic that does sit in an adapter sits mostly in `rustyfarian-esp-hal-ws2812`, whose host tests cannot build on macOS (see `docs/project-lore.md`).
  Before the retrofit, `long_frame_action`, `clk_divider_for` and the latch arithmetic were checked only by const asserts, and the ESP-IDF latch logic only on hardware.

## Decision

1. Shared logic uses the cheapest shape that fits.
   Flat logic (decision 3) comes first; the rungs below name the patterns beyond it.
   They are ordered by the indirection they add, not a progression: a rung-1 driver may contain a rung-2 decoder, and a rung-2 decoder never has to grow into a rung-3 machine.
   - **Rung 1 — generic driver.** The flow lives in a pure crate, generic over `embedded-hal` traits.
   - **Rung 2 — event machine.** The core decodes samples: `update(sample, now) -> Option<Event>`.
     A tier, an ISR, or an `embedded-hal` adapter feeds it.
   - **Rung 3 — action machine.** The core emits I/O requests: `poll(&mut self, input) -> Action`, and a thin loop per tier performs them.
2. A rung-3 action machine is evaluated only when a flow meets one of these triggers:
   - It must run under more than one execution model, such as blocking and async.
   - The tiers perform I/O outside `embedded-hal`, such as RMT channel transactions, but must share the order of the steps.

   A trigger is necessary, not sufficient.
   A machine is chosen only when flat helpers would still leave a duplicated or branching sequence of steps in the tiers.
   The retrofit below met both triggers, and flat helpers sufficed.
3. Flat logic stays as pure functions or small types in the pure crates, not machines.
4. Time is an input: logic in a pure crate never decides how time is read.
   The caller passes `now`, reports elapsed waits, or hands a flat helper a time provider that the helper may call lazily, as `PollBudget::spent` does to keep one clock read per 16 polls.
   Wake-ups are injected the same way.
   Timestamps passed to one helper share a clock domain and are monotonic; see the clock contract under Instances.
5. A rung-3 machine answers input that is invalid in its current state with an explicit protocol-error result, such as `Unexpected`, never with an action that reads as "nothing to do".
   `Idle` or `Pending` is reserved for valid states that need no I/O yet.
   Input after a terminal action is invalid too, until the machine is reset.
6. Every finite operation defines its success, failure, timeout and cancellation outcomes.
   Retry and polling cycles have explicit budgets, and each budget states whether it counts transitions or wall-clock time.
   The adapter bounds each I/O call where the underlying API allows it, and documents a call that can block where it does not.
   A reachable terminal action alone does not bound a loop.
   `PollBudget` is the instance here: a wall-clock deadline plus a count of unchanged clock reads.
7. Shared policy and hardware-independent decisions belong in a pure crate.
   Adapters keep resource ownership, hardware sequencing, synchronisation, cancellation handling and error mapping.
   A policy difference kept between tiers needs a written rationale, in the code or in this ADR.
   Review rule (from `rustyfarian-network` ADR 022 decision 10): a branch in a driver crate that is none of these is logic and moves into a pure crate.
   Driver logic goes to `bunting`, status-LED logic to `pennant`, animation logic to `ferriswheel`.
8. Per-chip behaviour is a runtime value passed in by the tier, not a `cfg` gate inside the logic.
   `long_frame_action` already takes the RMT block size (48 or 64 words) as a parameter.
9. Tier-specific facts stay in the adapters: RMT channel setup and transactions, critical sections and long-frame masking ([ADR 009](009-long-frame-policy-and-latch.md)), the ESP-IDF `send_and_wait` workaround, clock reads and busy-waits (`Instant::now`, `Ets::delay_us`), and the cycle-counted AVR bit-bang loop.
10. A port trait is narrow: one driver call or one event mapping per method, with no decisions.
    A generic runner over a wide port is not used, because the port would have to model the whole flow.
11. A one-time retrofit applies the review rule to existing code before the next release, followed by one hardware run across the affected flows.
    After the retrofit, moving a flow up a rung is demand-driven: the first change that needs it pays for it.

## Instances

- **Rung 1:** `pennant::SimpleLed<P: OutputPin>`, `RgbGpioLed` and `RgbPwmLed`.
  The AVR `Ws2812Spi<SPI: SpiBus, N>` is generic over `embedded-hal` too; its encoding already lives in `bunting::prerender_spi`, so it stays a thin adapter in the AVR crate.
- **Flat logic:** `bunting` (`rgb_to_grb`, `ns_to_ticks`, `prerender_spi`, `GridBuffer`), every `ferriswheel` effect, and `pennant::PulseEffect`.
- **Flat logic with time as an input:** the `bunting` `latch` module (`latch_deadline_us`, `remaining_us`, `latch_wait_us_from_ns`, `StallGuard`, `LatchWait`) and `long_frame` module (`long_frame_action`, `masked_us`, `tx_timeout_us`, `clk_divider_for`, `PollBudget`, `LongFramePolicy`).
  The AVR drivers share `bunting::checked_spi_frame_len` and `bunting::collect_truncated`.
- **Rung 2 and rung 3:** none today.

### Clock contract

- esp-hal: microseconds since boot from the esp-hal `Instant` (system timer), read once per use in `now_us()`; monotonic.
- ESP-IDF: `std::time::Instant`; the tier converts the remaining duration to nanoseconds before calling `latch_wait_us_from_ns`.
- Overflow is excluded by the platform contract: a `u64` microsecond count since boot lasts about 585,000 years, so `latch_deadline_us` and `PollBudget::start` add without saturating.
- A backward step is excluded by the same contract.
  If one occurred, it would lengthen a latch wait or delay a budget's deadline by the size of the step; a stopped clock is caught by `StallGuard`.

## Retrofit

Code extraction completed on 2026-11-04.
Hardware validation is pending and blocks the next release.
The branches that decision 7 named moved out of the driver crates into `bunting`; public types keep their paths through re-exports.
Neither flow needed a machine: the order of the steps is two or three calls long, so flat logic with time as an input covers it.

### Final shapes

- **Frame spacing (latch).**
  The arithmetic lives in `bunting::latch`, with `now` as input.
  Each tier keeps the clock read and the busy-wait.
  The ESP-IDF tier keeps nanosecond round-up and the cap through `latch_wait_us_from_ns`.
  The esp-hal tier keeps `saturating_sub` with no cap, and stall detection through `StallGuard` and `LatchWait`.
- **Long-frame decision and budgets.**
  `long_frame_action`, `masked_us`, `tx_timeout_us`, `clk_divider_for` and `PollBudget` live in `bunting::long_frame`.
  `PollBudget` takes `now` lazily through a closure, which preserves one clock read per 16 polls in the `#[esp_hal::ram]` polling loops.
  `masked_us` takes the tier's tick constants (`CodeTiming`) instead of recomputing from nanoseconds.
- **AVR.**
  The SPI buffer-fit check and the truncating collector for `SmartLedsWrite::write` are shared helpers.

### Deliberately left in the adapters

Each item is adapter work under decision 7, or a difference with a stated reason.

- **When the next frame may start (`next_frame_at`).**
  This is a transaction-lifecycle fact.
  The blocking esp-hal and the ESP-IDF paths stamp `now + 280 µs` after the transmit returns.
  The async await path encodes the latch in its end code, so it stamps `now`, and a drop guard stamps on cancellation, which only the async adapter can observe.
- **Latch wait versus the `ChannelLost` check.**
  The blocking `do_transmit` waits for the latch and then fails with `ChannelLost`; `do_transmit_async` checks first.
  There is no functional reason for the difference: on a driver that has already lost its channel, the blocking path spends at most one 280 µs wait before the error.
  It was kept so the retrofit stays behaviour-preserving, and the blocking path is aligned when it is next changed.
- **The GRB send order in the AVR assembly loop.**
  Cycle-counted hardware sequencing.
- **The bench oracles.**
  Example code, not library code.

### Hardware validation

The hardware run required by decision 11 is the final hardware pass in `release-plan.md`, across the full chip matrix; it blocks the next release.
The evidence so far, by scope:

- **Helper tests** in `bunting` verify the arithmetic and the local state of `StallGuard`, `LatchWait` and `PollBudget`, including one clock read per 16 polls.
- **Target compilation** (`check-hal`, `check-hal-xtensa`, `check-idf`, `check-avr-target`) verifies that every tier builds against the helpers.
- **Linked-binary inspection** of the release `hal_c3_frame_sweep_async` ELF on the ESP32-C3 verified that the `#[esp_hal::ram]` polling functions call no `bunting` symbol and match `main` in size and call sites.
  It does not cover other chips or examples, and those functions already called flash-resident code on `main`, which is an open item in `release-plan.md`.
- **Adapter ordering** is verified by review only: the driver crates cannot run host tests, so there are no adapter sequencing tests.
- **The waveform** needs the hardware pass.

## Consequences

### Positive

- The latch and long-frame logic gain host tests, which closes the roadmap item on host tests for `PollBudget` and the latch.
- The frame-spacing arithmetic of both ESP drivers lives in `bunting`; each driver keeps its own rounding rule, as before.
- Driver crates shrink to I/O and error mapping, which the review rule lets `code-reviewer` check.
- The repo shares one vocabulary with its siblings, so a reviewer can compare them point by point.

### Negative

- `bunting` gains driver-policy types such as `LongFramePolicy`, which only the esp-hal tier uses today.
- Host tests verify helper arithmetic and state, not adapter ordering or the waveform; the long-frame benches remain the arbiter.
- Constraints on the tier loop, such as running inside a critical section or busy-waiting with interrupts masked, are not checked by any type.

### Constraints

- The pure crates stay `no_std` and allocation-free; machines are free of async, and their inputs and actions are small `Copy` values.
- Rung 2 and 3 machines need state-transition tests in the pure crate that owns them.
- A change to the latch or long-frame logic re-runs the long-frame bench ([`docs/features/long-frame-hardware-validation-v1.md`](../features/long-frame-hardware-validation-v1.md)).

## Alternatives Considered

| Alternative                   | Pros                                | Cons                                                       | Why Rejected                                                         |
|:------------------------------|:------------------------------------|:-----------------------------------------------------------|:---------------------------------------------------------------------|
| Adopt power ADR-004 verbatim  | Identical wording to one sibling    | Its "no generic runner over a port" conflicts with network | Decision 10 (narrow ports) reconciles both and is taken here instead |
| Repo-specific pattern         | Tuned to RMT drivers and effects    | Rules and vocabulary drift from the siblings               | The ladder fits as is; only the instances differ                     |
| Action machines everywhere    | One uniform shape                   | Ceremony for effects, encoding and `embedded-hal` drivers  | No drift to fix; adds indirection for nothing                        |
| Logic stays in the adapters   | No moves, no re-exports             | The latch arithmetic exists twice; esp-hal logic untested  | Leaves the drift and the macOS host-test gap open                    |

## References

- [ADR 003](003-rename-core-to-pure.md), [ADR 005](005-dual-hal-strategy.md), [ADR 009](009-long-frame-policy-and-latch.md), [ADR 010](010-interchangeable-driver-semantics.md), [ADR 011](011-cross-crate-driver-api-policy.md) — the decisions this one builds on
- `rustyfarian-network` ADR 023 (`docs/adr/023-shared-logic-ladder.md`) and ADR 022 decision 10 — the origin of the ladder
- `rustyfarian-power` ADR-004 (`docs/adr/004-shared-logic-ladder.md`) — the sibling ladder in the power repo
- `rustyfarian-peripherals` ADR-010 (`docs/adr/010-shared-logic-ladder.md`) — the sibling ladder in the peripherals repo
