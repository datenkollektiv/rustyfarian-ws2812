# Roadmap

*Last updated: November 2026*

The April 2026 `esp-hal` release wave (`v0.5.0`) is shipped, the AVR bit-bang
driver is the recommended backend (per ADR 007), the GPIO8 RMT hang is
resolved upstream, and the Xtensa ESP32 / WROOM-32 bare-metal target is
verified clean under `esp-hal 1.2.2`.
The September 2026 wave (`esp-hal 1.2.2`, `esp-rtos 0.4.0`, MSRV 1.95) is
compile-verified on ESP32-C3, ESP32-C6, ESP32-WROOM-32, and ESP32-S3, with
hardware-validated results on C6 and C3 — see
[`docs/features/archive/esp-hal-stack-upgrade-september-2026-v1.md`](features/archive/esp-hal-stack-upgrade-september-2026-v1.md) —
and on ESP32-S3 (CrowPanel 1.28") — see
[`docs/features/archive/esp-hal-esp32s3-support-v1.md`](features/archive/esp-hal-esp32s3-support-v1.md).
All six crates are on crates.io: the pure-logic trio (`bunting`, `pennant`,
`ferriswheel`) since `0.5.0`, and all three driver crates
(`rustyfarian-avr-ws2812`, `rustyfarian-esp-idf-ws2812`,
`rustyfarian-esp-hal-ws2812`) since the `0.6.0` wave.
The 2026-05-05 vision review confirmed AVR as a first-class supported MCU
family and ruled in-workspace networking demos (e.g. ESP-NOW) out of scope.
All three embedded targets (RISC-V, AVR, Xtensa) are now gated in CI by path-filtered
cross-target workflows.
For the unreleased 0.8, the esp-hal and ESP-IDF drivers are interchangeable and the API is streamlined across all platforms.
The near-term priorities are hardware validation of the long-frame paths on the current tree,
tightening the `cargo-deny` configuration, and the hardware run for the shared-logic ladder retrofit (ADR 012).
Mid-term priorities are removing the `esp-idf-hal` `send_and_wait` workaround
when the upstream fix lands, `SmartLedsWriteAsync` for the async driver, and
scoping `MAX_LEDS` correctly.
All completed items are documented in the [CHANGELOG](../CHANGELOG.md).

```mermaid
%%{init: {
  "theme": "base",
  "themeVariables": {
    "cScale0": "#e8f5e9",
    "cScaleLabel0": "#2e7d32",
    "cScale1": "#c8f7c5",
    "cScaleLabel1": "#1b5e20",
    "cScale2": "#fff3cd",
    "cScaleLabel2": "#7a5a00",
    "cScale3": "#e3f2fd",
    "cScaleLabel3": "#0d47a1"
  }
}}%%

timeline
    title Fuzzy Rustyfarian WS2812 Roadmap

    Near term : Logic-analyser captures of the frame sweeps
              : cargo-deny — ban multiple rgb versions
              : Bench with place_rmt_driver_in_ram on
              : Downstream C3 Wi-Fi re-bench

    Mid term  : Remove send_and_wait workaround (esp-idf-hal fix)
              : SmartLedsWriteAsync for esp-hal async driver
              : Scope MAX_LEDS and fix positional effects for strips > 256 LEDs
              : Add scope guard in grid.rs (README row done)
              : Document async support status in README driver table

    Long term : Property tests for pure crates
              : Track rgb 0.9 migration
              : ferriswheel SemVer compatibility statement
              : Upstream contribution evaluation
              : embedded-graphics-core evaluation
              : Monitor esp-idf-hal for async RMT support
              : Upstream issues — esp-hal ISR refill, esp-idf-hal IRAM handle_isr
```

---

## Ecosystem Integration

### Recurring: audit `deny.toml` exceptions

Each ignored advisory or per-crate licence exception in [`deny.toml`](../deny.toml)
should be re-checked periodically — the underlying upstream bug, deprecation, or
licensing situation may have been resolved, in which case the exception can be
removed and the dep graph cleans up.

Current entries (as of 2026-05):

- `RUSTSEC-2024-0436` — `paste` unmaintained; a **direct** dependency of `esp-hal 1.2.2`
  as well as transitive through `riscv 0.15.0`. Bumping `riscv` alone therefore cannot
  clear it — `esp-hal` must drop `paste` first. Re-checked 2026-09-21 against 1.2.2
  (still present); re-run `cargo tree -i paste` after each `esp-hal` bump.

(The `bare-metal` / `atdf2svd` exceptions previously needed for `rustyfarian-avr-ws2812`
were eliminated in `v0.5.0` by switching the bit-bang backend to raw `cli` + `SREG`
save/restore inline asm, dropping the `avr-device` dependency entirely.)

Cadence: revisit at every minor release, or when a new exception is added.
Mechanism: walk each entry, run `cargo update` against the relevant upstream, and
attempt to remove the exception.
If the advisory or licence still fires, leave it in place but refresh the rationale
comment with the current upstream state.

### Add `multiple-versions = "deny"` for `rgb` in `deny.toml`

`ferriswheel` includes a compile-time type-identity assertion that fails the
build if its `rgb` and `smart-leds-trait`'s `rgb` resolve to different versions.
That assertion is belt-and-braces for downstream consumers who don't run `cargo-deny`.
The root-cause prevention is a `[bans]` rule that fails CI before the assertion
ever fires:

```toml
[bans]
multiple-versions = "deny"
# add explicit [[bans.allow]] entries with rationale for any known-safe duplicates
```

Add an explicit `allow` entry for any crate that legitimately ships multiple
versions in the dep graph at the time of the change.

---

## Hardware Driver Improvements

### Logic-analyser captures of the frame sweeps

Capture `hal_c3_frame_sweep_async` on GPIO4 and `hal_c6_frame_sweep_async` on GPIO18 for at least three cycles (`maintenance-plan.md`, long-frame async release gate).
Check 24 × LEDs symbols per frame, T0H/T1H in spec, no stale bits and ≥ 280 µs latches, and measure the 12- and 64-LED masked frames against `masked_us` (expected 317–375 µs and 1690–1997 µs).
Deferred from v0.9.0 (decided 2026-11-07); both serial halves passed on the release binaries (C3 3 cycles, C6 12 cycles).
Carried open question: force the drop-and-rebuild path after `Error::ChannelLost` on hardware, or keep the "rebuild not hardware-tested" caveat.

### Bench the long-frame paths with `place_rmt_driver_in_ram` on

The v0.9.0 desk check found the `#[esp_hal::ram]` poll loops calling esp-hal's RMT `poll` from flash on every iteration, because the option defaults to off.
Build the frame sweeps with `ESP_HAL_CONFIG_PLACE_RMT_DRIVER_IN_RAM=true`, confirm by disassembly that the poll path is RAM-resident, and compare the frame-sweep results; then decide whether the examples or the README should turn it on.

### Downstream C3 Wi-Fi re-bench of the long-frame paths

Re-run R1 (default `set_pixels_slice`), R2 (`SmartLedsWrite::write`), R3 (`Poll`), R4 (ceiling lifted) and R5 (`set_pixels_slice_masked`), plus the async path, under Wi-Fi on a C3 in a downstream project.
The in-repo long-frame checks run in the v0.9.0 hardware pass; these were split off on 2026-11-07 because they need the downstream radio bench.

### Add esp-hal rainbow examples, so the bench `rainbow` check covers both tiers

The plain rainbow exists only as an ESP-IDF example (`idf_<chip>_rainbow`), so the bench's `rainbow` check runs ESP-IDF only, while `pulse` runs esp-hal.
Add `hal_<chip>_rainbow` for S3, C3, C6 and ESP32 with the same `RainbowEffect` settings as the IDF examples, and list each one before its IDF twin in `bench/<chip>/rainbow.toml`.
Deferred past v0.9.0 (decided 2026-11-07): new examples would need their own hardware run before that release.

### Document async support status in README driver table

`rustyfarian-esp-hal-ws2812` has an `async` feature with `AsyncStatusLed`.
`rustyfarian-esp-idf-ws2812` does not, and currently a user has to read the
source to discover this.

Add a column or note to the README driver table marking async as esp-hal-only
by design: ESP-IDF users spawn threads; Embassy is the async runtime for
bare-metal only (see ADR 008).
If a future `esp-idf-hal` release adds async RMT (tracked below), this entry
can be updated.

### Remove `send_and_wait` workaround when `esp-idf-hal` fixes `EncoderWrapper`

`esp-idf-hal 0.46.2` has a bug in `EncoderWrapper`: the `From<rmt_encode_state_t>` conversion
panics on bitwise-OR'd flag values (e.g. `COMPLETE | MEM_FULL = 0x03`) that the C encoder
legitimately returns.
Re-checked 2026-09-21 against the `esp-idf-hal 0.47.0` source: the `_ => panic!` arm in
`src/rmt/encoder.rs` is unchanged, so the workaround is still required on 0.47.
Since the encode callback runs in ISR context, the panic triggers `abort()`.
We work around this by using `start_send` + `wait_all_done` directly with the C-side
`BytesEncoder`, bypassing the Rust `EncoderWrapper` entirely.
When a future `esp-idf-hal` release fixes this (likely by treating `rmt_encode_state_t` as
a bitfield rather than an enum), switch back to `send_and_wait` and remove the `transmit_bytes`
helper and its `unsafe` block.
Track: [esp-idf-hal GitHub issues](https://github.com/esp-rs/esp-idf-hal/issues).

### Implement `SmartLedsWriteAsync` for `Ws2812Rmt<'d, Async, N>`

The `smart-leds-trait` ecosystem defines a `SmartLedsWriteAsync` trait for async LED writers.
`rustyfarian-esp-hal-ws2812` already implements `SmartLedsWrite` for the blocking variant.
Adding `SmartLedsWriteAsync` for the async variant completes the `smart-leds` integration
and allows users to apply `brightness()` and `gamma()` adapters in async contexts.
Blocked on: `SmartLedsWriteAsync` trait stabilisation in the `smart-leds` ecosystem.
See [ADR 006](adr/006-async-support.md) and [ADR 008](adr/008-embassy-as-async-runtime.md).

---

## Animation Effects & Pure Crates

The current `ferriswheel` crate provides more than a dozen well-tested, ring-specific effects:
`RainbowEffect`, `PulseEffect`, `BreatheEffect`, `SpinnerEffect`, `MeteorEffect`, `TwinkleEffect`, `FireEffect`, `CylonEffect`, `KnightRiderEffect`, `ChaseEffect`, `FlashEffect`, `ProgressEffect`, `SectionEffect`, and `RainbowCometEffect`.

### Scope `MAX_LEDS` and fix positional effects for strips > 256 LEDs

Two coupled constraints that must move together:

**`MAX_LEDS = 256` is a crate-wide cap** whose rationale (256 distinct hues for `RainbowEffect`)
only applies to that one effect.
For `FireEffect` or `MeteorEffect` on a 300-LED strip — common for room lighting — the limit
is artificial and surprising.
Resolution: move the hue-count rationale to `RainbowEffect::MAX_USEFUL_HUES` and raise or
remove the crate-wide cap.

**`position: u8` in all positional effects** (`SpinnerEffect`, `ChaseEffect`, `RainbowEffect`,
`MeteorEffect`) and in `advance_position` — safe only while `MAX_LEDS ≤ 256`; any increase
silently truncates positions.
Fix: change `advance_position` and all `position` fields to `usize`.

These two items must move together: raising `MAX_LEDS` without fixing the `u8` positions is
a soundness bug.

### Add a scope-guard comment to `grid.rs`

`bunting` 0.5.0 added `GridBuffer` / `GridLayout` / `GAMMA_2_0`.
The README crate-table half of this item is done — the `bunting` row now mentions `grid`.

Remaining: a `// SCOPE:` comment at the top of `grid.rs` quoting the VISION non-goal
("matrix-first animation vocabulary is out of scope") to guard against feature creep.

### Deferred follow-ups

Small improvements deferred during reviews.
Not blocking, but tracked here to avoid being lost.

- **`MeteorEffect` decay math: `/255` vs fixed-point `>> 8`** — current `brightness * decay / 255`
  maps decay values directly to percentages and keeps `decay=0` = instant black.
  The `* (decay + 1) >> 8` fixed-point variant is marginally faster on bare metal but changes
  the semantics of `decay=0` (near-zero, not instant black), requiring test updates.
  Revisit only if a performance need or a `with_decay_pct(f32)` builder is added.

- **`#[must_use]` audit for builder and getter methods in the pure crates** — `with_*` builders
  and `Result`-returning constructors silently discard work if the return value is dropped.
  Sweep `bunting`, `pennant`, and `ferriswheel` and add `#[must_use]` where it guards a real mistake.

- **Fixed `[u8; MAX_LEDS]` state arrays in `TwinkleEffect` and `FireEffect`** — a 12-LED ring
  still carries 256-byte heat/brightness arrays (~340-byte structs).
  Acceptable trade-off today; revisit with const-generic sizing if RAM pressure appears on AVR.

- **`impl Effect` delegation boilerplate repeated across all 14 effects** — each effect hand-writes
  the same three-method trait impl delegating to inherent methods (the documented recursion-avoidance
  pattern).
  A small macro could remove ~150 lines; weigh against the readability cost of macro indirection.

- **Tail-length clamping is inconsistent across effects** — `SpinnerEffect::with_tail_length` clamps
  via `min(num_leds)`, `MeteorEffect::new` clamps inline with `6.min(num_leds - 1)`, and a
  `clamp_tail_length` util exists in `util.rs`.
  Consolidate on the util function; behaviour-preserving refactor.

- **Buffer-validation order differs between effects' `update` paths** — most validate via the
  leading `current()` call, `FlashEffect` mutates state after that call.
  All paths are currently correct (validation happens first either way); unify the pattern so the
  invariant is structural rather than incidental.

### FireEffect follow-up

- **Gradient parameterisation** — `with_gradient(&'static [GradientStop])` for users who want a custom palette (e.g. blue ice, purple plasma). Requires a `GradientStop` type and piecewise-linear interpolation in `fire_color`. `no_std`-safe with a fixed-size slice; `Vec` is off the table.

---

## Long-term / Strategic

### Add property tests for pure crates (`bunting`, `pennant`, `ferriswheel`)

The unit tests are good, but for colour math (`hsv_to_rgb`, `lerp_color`, `scale_brightness`)
and effect invariants (e.g. "`update` then `current` returns the same buffer", "`reset` returns
to the same state as a fresh `new()`"), `proptest` or `quickcheck` would catch a class of bugs
that unit tests won't.
The `FireEffect` modulo-bias bug fixed in 0.5.0 is exactly the shape of bug a property test
would surface earlier.
Low urgency — the architecture is already test-friendly on the host target.

### Track `rgb 0.9` migration

`rgb 0.9` redesigns `RGB8` from a struct with public `r`/`g`/`b` fields to a wrapper around
`[u8; 3]` with accessor methods.
The workspace is pinned to `0.8`.
Every downstream consumer that pulls in `rgb 0.9` (via another dep) will hit either a
duplicate-version warning or a hard incompatibility through the type-identity assertion.

Defensible position: "migrate when `esp-hal` moves to `rgb 0.9`."
That position should be explicit rather than implicit.
File a tracking issue; re-evaluate at each `esp-hal` minor release.

### Add SemVer compatibility statement to `ferriswheel`

`ferriswheel` is at `0.5.0` — pre-1.0 in Cargo SemVer means every minor bump can break.
With 14 effects, an `Effect` trait with `PartialEq` derives, and `MAX_LEDS` as public API,
there is real surface area worth committing to before third-party consumers pin the crate.

Add a section to the crate-level rustdoc (or a `STABILITY.md`) along the lines of:

> The `Effect` trait is the stable interface.
> Concrete effect structs may gain new builder methods between minor versions;
> existing builder methods will only be removed in major bumps.
> The `MAX_LEDS` constant is part of the public API.

Do this before publishing `ferriswheel` to new crates.io consumers.

### Evaluate upstream contribution to `smart-leds-rs`

The pure-logic crates (`bunting`, `ferriswheel`) represent a gap in the ecosystem:
no existing `smart-leds-rs` crate provides ring-geometry animations testable without hardware.
Once the APIs are stable, evaluate whether proposing these as upstream additions or companion crates makes sense.
Decision should follow a stability review and user feedback.

### Monitor `esp-idf-hal` for async RMT support

`rustyfarian-esp-idf-ws2812` is blocking-only because `esp-idf-hal`'s `TxChannelDriver`
has no async transmit API.
Re-checked 2026-09-21: `0.47.0` added only an async `wait_for_progress`; `start_send`,
`send_iter` and `send_and_wait` remain synchronous.
If a future `esp-idf-hal` release adds async RMT (likely using `esp-idf-svc`'s executor or
`tokio`, not Embassy), async support can be added to the IDF driver under a separate feature flag.
This would be a different runtime from the HAL driver's Embassy-based async (see [ADR 008](adr/008-embassy-as-async-runtime.md))
— the two drivers already diverge on error handling (ADR 005), so runtime divergence is expected.
Track: [esp-idf-hal releases](https://github.com/esp-rs/esp-idf-hal/releases).

### File upstream RMT issues

- **esp-hal: refill TX RAM in the RMT ISR** — esp-hal 1.2.2 refills from the executor, which is why long frames need the critical-section policy ([ADR 009](adr/009-long-frame-policy-and-latch.md)); an ISR refill would let the policy go.
- **esp-idf-hal: place `handle_isr` in IRAM**, or skip registering it as `on_trans_done` when no async wait is used; until then `CONFIG_RMT_ISR_IRAM_SAFE` breaks `Ws2812Rmt::new` ([ADR 010](adr/010-interchangeable-driver-semantics.md)).

Draft both for the maintainer to post.

### Evaluate `embedded-graphics-core` integration for matrix displays

`ws2812-esp32-rmt-driver` demonstrated a clean `embedded-graphics-core` drawing target
for addressing LEDs as a 2D pixel grid.
If a matrix display use-case emerges, this pattern provides a ready-made approach.
Not a near-term priority — track as a future option.
