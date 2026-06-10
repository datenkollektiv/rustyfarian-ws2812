# Maintenance Plan

Regular maintenance workbook for `rustyfarian-ws2812` — a Cargo workspace
of `no_std` embedded Rust crates targeting ESP32 (C3, C6, WROOM-32) and AVR
(ATmega328P) for WS2812 / NeoPixel LED control.

Covers: build verification, dependency updates, security scanning, CI/CD
status, hardware-target validation, and toolchain freshness.

The first end-to-end cycle ran on 2026-04-29; the artefacts in
[`audit/`](audit/) serve as concrete templates for the audit / plan /
maintenance file structure.

## Build & Test

### Primary build gate
- `just verify` — non-modifying full check: fmt-check, deny, check, lint, test.
  Use this as the audit's primary build gate. Exit 0 means fmt + deny + check +
  clippy + all unit/doc tests passed on the host target.

### Compile-side validation for hardware-target drivers
- `just check-hal` — bare-metal `esp-hal` driver against `riscv32imac-unknown-none-elf`.
- `just clippy-hal` — clippy on the same target with `-D warnings`.
- `just check-idf` — ESP-IDF driver (requires `espup` toolchain).
- `just check-avr` — AVR driver on host target (no toolchain required).
- `just check-avr-target` — AVR driver against real `avr-none` (requires nightly + `avr-gcc`).

### Example compile checks (when API changes are suspected)
After any `esp-hal` minor bump, spot-check at least one example per chip family
**and** one per feature dimension, since example code lives outside `lib.rs`:
- `just build-example hal-ws2812 hal_c6_pulse` (blocking, C6)
- `just build-example hal-ws2812 hal_c6_pulse_async` (async, C6)
- `just build-example hal-ws2812 hal_c6_multitask_async` (Embassy spawn / sync — most likely to break)
- `just build-example hal-ws2812 hal_c6_smart_leds` (smart-leds-trait integration)
- `just build-example hal-ws2812 hal_c3_pulse` (C3)
- `just build-example hal-ws2812 hal_esp32_pulse` (WROOM-32)

### Hardware tests
End-to-end animation validation requires real boards. Connect over USB,
then `just run <example>` builds, flashes, and opens the serial monitor.

Pass criteria (required for "hardware-target validation" to pass):
- LEDs render the expected pattern and colour order for the selected example (no channel swap, e.g. GRB/RGB mismatch).
- No visible flicker, random flashes, or frame tearing during a continuous 60-second run.
- Brightness changes and animations are smooth and repeatable across at least 3 consecutive runs.
- Serial monitor shows normal startup/output only (no panic, watchdog reset, backtrace, or repeated error logs).
- Board remains stable for the full run (no unexpected reboot/disconnect), and rerunning the same command yields the same visual result.

At minimum, exercise:
- `just run hal_c6_pulse` — basic blocking RMT on C6.
- `just run hal_c6_multitask_async` — exercises the migrated Embassy task-spawn
  and the multi-task render+button architecture.
- `just run hal_c3_pulse` and `just run hal_esp32_pulse` if those boards are present.

If a previously-known-broken edge case has documented lore (e.g. a specific pin
or setting), retest it on the new stack — upstream releases sometimes resolve
latent bugs as a side effect (e.g. the GPIO8 hang was fixed by `esp-hal 1.1.0`
without any driver change on our side). Hardware tests done in 2026-04-29's
cycle confirmed the upgraded ESP32-C6 driver works with the onboard
SK68XXMINI LED on GPIO8.

## Dependency Updates

### Workspace `Cargo.toml`
All shared dependency versions are declared in the root `[workspace.dependencies]`
section. Each entry should have a comment explaining the constraint and any
coordination requirement.

Cargo treats `"1.0"` as `^1.0` and `"=1.0.0"` as exact pinning — comments must
reflect the actual semantics, not the intent.

### External crates to track (host-buildable)
- `rgb`, `smart-leds-trait`, `smart-leds`, `embedded-hal` — stable embedded ecosystem,
  patch-level bumps usually safe; covered by host tests.
- `anyhow` — std-only, used by ESP-IDF examples.
- `esp-idf-hal` — IDF stack, gated by `esp` toolchain. Patch-level bumps within
  the same minor are typically safe.

### `esp-hal` stack (the bigger upgrade story)
- `esp-hal`, `esp-rtos`, `esp-bootloader-esp-idf`, `esp-println`
  — released as a coordinated monorepo wave; treat them together, not piecemeal.
  Other crates from the same wave (`esp-radio`, `esp-alloc`, etc.) come into
  scope when a feature actually consumes them.
- The pre-1.0 crates' minor bumps typically signal breaking API changes;
  expect to fix call sites in our examples.
- Each coordinated upgrade should produce its own feature doc under
  `docs/features/esp-hal-stack-upgrade-<period>-v1.md` documenting the
  decisions, constraints, migration steps, and verification outcomes.
  See [`docs/features/esp-hal-stack-upgrade-april-2026-v1.md`](docs/features/esp-hal-stack-upgrade-april-2026-v1.md)
  as a worked example.

### Embassy crates
- `embassy-time`, `embassy-executor`, `embassy-sync` — workspace constraints must
  match what `esp-rtos` pulls transitively. Update them as part of any `esp-rtos`
  bump, not independently.
- `esp-sync` (published from Espressif's `esp-hal` monorepo, but consumed as
  internal plumbing by the upstream crates rather than directly by this
  workspace) deliberately depends on
  multiple `embassy-sync` versions for backwards-compat shims. `Cargo.lock`
  resolving multiple `embassy-sync` versions is **expected** and not a defect;
  application code resolves to the single workspace-pinned version.

### Coordinated `esp-hal` upgrade — runbook
This is the recurring high-effort upgrade. Apply the same sequence each time
a new wave appears upstream:

1. **Survey.** Query crates.io for the latest version of every crate in the
   `esp-rs` monorepo wave. Note that `esp-hal` itself often follows the
   smaller crates by a week.
2. **Spec.** Create or update a feature doc:
   `docs/features/esp-hal-stack-upgrade-<period>-v1.md`. Include the version
   table, decisions, constraints, migration steps, and explicit open questions
   (e.g. "Do any breaking API changes affect our examples?", "Are bare-metal
   `Cargo.lock` `embassy-sync` versions consistent after upgrade?").
3. **Bump.** Update workspace `Cargo.toml` constraints for the six esp-* crates
   in one diff. Then `just update`. Inspect `Cargo.lock` for the new resolved
   versions.
4. **Embassy alignment.** After the esp-* bumps, identify the new transitive
   versions of `embassy-time`, `embassy-executor`, `embassy-sync` and update the
   workspace constraints to match. Run `just update` again.
5. **Compile check.** Run `just check-hal` and `just clippy-hal`. Library code
   often passes immediately; example code is where API breakage surfaces.
6. **Example fix-up.** Run `just build-example hal-ws2812 <example>` for at least
   one async example (most likely to break) and one per chip family. Apply
   migration patches as needed. Common upstream change patterns observed so far:
   - **`esp-hal` RMT builder reshapes** — pin/config split; check `configure_tx` /
     `with_pin` signatures across all `hal_*` examples.
   - **`embassy-executor` task-spawn API** — `Spawner::spawn` return type and the
     `#[embassy_executor::task]` macro's wrapper around the function's return.
   For multi-site mechanical migrations, batch text edits via
   `sed -i ''` or `perl -i -0pe` are usually faster than per-file edits.
7. **Doc refresh.** Add a new section to
   [`docs/esp-hal-version-matrix.md`](docs/esp-hal-version-matrix.md) for the
   new wave; the doc is evergreen and grows by appending. Update version refs
   in `docs/ROADMAP.md` and `docs/project-lore.md`.
8. **Re-audit.** Re-run `just audit` and `just deny` on the upgraded graph.
   Re-evaluate any ignored advisories in `deny.toml` (notably `paste` via `riscv`).
9. **Hardware retest.** See "Hardware tests" above. Always retest at least
   `hal_c6_pulse` and `hal_c6_multitask_async`. Take the chance to retest any
   open hardware-edge-case lore items — upstream may have fixed them silently.
10. **Changelog + commit.** Update `CHANGELOG.md ## [Unreleased]` with one bullet
    per crate bump and one per applied API migration. Branch convention:
    `<period>-release-wave` (e.g. `april-2026-release-wave`).

### Update strategy
- **Conservative for `esp-*` crates**: minor bumps in this stack typically break
  example call sites; always verify with `just check-hal`, `just build-example
  ...`, and a hardware re-test where possible.
- **Liberal for pure-logic crates** (`rgb`, `smart-leds`, `smart-leds-trait`):
  patch and minor bumps are usually safe; covered by host tests.

### Security scanning
- `just deny` — runs `cargo deny check` (advisories, licenses, bans).
- `just audit` — runs `cargo audit` (RustSec advisory DB).
- `deny.toml` lists currently-ignored advisories with rationale.
- After every `esp-hal` upgrade, re-evaluate the `paste` advisory ignore — `riscv`
  may eventually move off `paste`, at which point the ignore can be removed.
  Use `just audit`'s "Dependency tree" output to confirm.

## CI/CD

GitHub Actions workflows in `.github/workflows/`:
- `audit.yml` — RustSec advisory check on push.
- `clippy.yml` — lint gate.
- `fmt.yml` — formatting gate.
- `rust.yml` — build + test matrix.

Local equivalents: `just act-audit`, `just act-clippy`, `just act-fmt`, `just act-ci`,
`just act-all` (require Docker + `act`).

## Documentation Freshness

Documents that should be reviewed for staleness during quarterly cycles:
- [`docs/esp-hal-version-matrix.md`](docs/esp-hal-version-matrix.md) — evergreen
  doc; append a new "Current State" section per release wave, keep historical
  sections below.
- [`docs/project-lore.md`](docs/project-lore.md) — review entries for current
  accuracy; remove any that have been resolved upstream (e.g. the GPIO8 hang
  was removed in 2026-04-29's cycle after `esp-hal 1.1.0` resolved it). If the
  file exceeds ~10 entries / ~50 lines of entry content, run the `/project-lore`
  maintenance skill ("condense").
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — verify items still reflect priorities;
  remove resolved items from both the timeline and the body sections.
- [`CHANGELOG.md`](CHANGELOG.md) — confirm `## [Unreleased]` matches actual
  branch state; entries land here as work happens, not all at release time.

## Scheduled Maintenance Cadence

### Monthly
- [ ] `just verify` builds clean.
- [ ] `just audit` produces no new advisories not already in `deny.toml`.
- [ ] `just deny` passes (licenses, advisories, bans).
- [ ] Review GitHub workflow run status (`audit.yml` weekly schedule).
- [ ] Patch-level bumps of pure-logic dependencies (`rgb`, `smart-leds*`, `embedded-hal`).

### Quarterly
- [ ] Everything in the monthly checklist.
- [ ] Audit `[workspace.dependencies]` entries against latest registry versions
      (e.g. `cargo search <crate>` or check each crate's page on crates.io).
- [ ] Detect any `esp-hal` monorepo release wave; if present, follow the runbook
      under "Coordinated `esp-hal` upgrade" above.
- [ ] Hardware re-test of at least one representative `hal_*` example per chip
      family that's available; always include `hal_c6_multitask_async`.
- [ ] Review `docs/project-lore.md` entries for accuracy; remove resolved items.
- [ ] Refresh `docs/esp-hal-version-matrix.md` if the stack moved.
- [ ] Verify LED/effect contract assumptions:
  - [ ] Confirm `MAX_LEDS = 256` is still valid for all supported targets.
  - [ ] Confirm `Effect` trait method signatures still match current implementations and examples.
  - [ ] Confirm buffer-size assumptions remain consistent across trait, implementations, and examples.
  - [ ] Confirm any `Effect` trait bounds still match current implementations and examples.
- [ ] Re-evaluate ignored advisories in `deny.toml`.

## Maintenance Protocol

Each maintenance cycle produces three files in `audit/`:
1. `YYYY-MM-DD-<cadence>-audit.md` — read-only assessment.
2. `YYYY-MM-DD-<cadence>-plan.md` — executable plan derived from the audit.
3. `YYYY-MM-DD-<cadence>-maintenance.md` — record of what was actually applied,
   including outcomes for any deferred items resolved during the cycle.

The `audit/` directory is **git-ignored** — these are internal maintenance logs
not intended for public consumption. Behavioural changes that result from a
maintenance cycle (dependency bumps, API migrations, bug fixes) belong in
`CHANGELOG.md ## [Unreleased]`; deferred or recurring concerns belong in
`docs/ROADMAP.md`; non-obvious technical insights belong in `docs/project-lore.md`.

The 2026-04-29 quarterly cycle's three files demonstrate the structure end-to-end.
Because `audit/` is git-ignored, do not rely on those local files for bootstrap in
a fresh clone; use the following minimal templates instead:

- `YYYY-MM-DD-<cadence>-audit.md`
  - Scope and cadence (`monthly` / `quarterly`)
  - Toolchain + environment snapshot
  - Findings by area (build, deps, security, CI, hardware)
  - Risk/impact summary and recommended actions
- `YYYY-MM-DD-<cadence>-plan.md`
  - Ordered action list derived from the audit
  - Preconditions, commands to run, and expected outcomes
  - Rollback/deferral notes per action
- `YYYY-MM-DD-<cadence>-maintenance.md`
  - Actions actually executed (with timestamps if useful)
  - Command outputs/results and any deviations from plan
  - Deferred items and follow-up ownership/location

A typical cycle takes 2–4 hours of focused work for a coordinated `esp-hal` wave
(longer the first time encountering each new upstream API change), or under
30 minutes for a clean monthly pass.
