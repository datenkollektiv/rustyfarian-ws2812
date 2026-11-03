# Feature: Cross-target CI — RISC-V, AVR, Xtensa

*Status: Implemented and green on CI (2026-08-13) — all three jobs pass; see the caveat on path-filter exclusion in State.*

Gate every embedded target this project supports in CI.
Before this change **no cross-compilation was gated at all**: `verify`, `pre-commit`, `ci` and all
four GitHub workflows ran `ubuntu-latest` + stable toolchain over the host-target pure crates only.
An upstream break in `esp-hal`, `avr-hal`, or a toolchain regression could rot a driver silently
until someone happened to build it by hand.

## Decisions

| Decision                                                           | Reason                                                                                                                                                                                                                                                                                         | Rejected alternative                                                                                                                                                         |
|:-------------------------------------------------------------------|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|:-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Three separate workflow files, one per target                      | GitHub Actions `paths:` filters are **workflow-level, not per-job**. Per-target filtering in a single file would need a third-party action (e.g. `dorny/paths-filter`) plus a fan-out job. Three files keep filtering native and add no supply-chain surface to a repo that runs `cargo-deny`. | One `cross-targets.yml` with three jobs (cannot filter per job); one workflow with a union path filter (would run the expensive Xtensa job on AVR-only changes)              |
| Call existing `just` recipes rather than inlining `cargo` commands | Single source of truth — CI and local runs execute identical commands. Every recipe needed already existed bar one.                                                                                                                                                                            | Inline `cargo` invocations in YAML (the shape the roadmap originally proposed), which would drift from the justfile                                                          |
| AVR job builds the **example package**, not just the driver crate  | `rustyfarian-avr-ws2812` is lib-only, so `cargo build` on it emits an rlib and never links. The AVR linker, `-C target-cpu=atmega328p` and the AVR ABI are only exercised when the example binaries link. Measured: 4 linked ELFs in 15.1s.                                                    | Driver-crate build only — compiles clean while leaving exactly the linker/ABI rot this feature exists to catch                                                               |
| Xtensa toolchain pinned to **1.97.0.0**                            | Matches what an unpinned `just setup esp` installs locally today, so CI and a fresh dev environment agree.                                                                                                                                                                                     | 1.95.0.0 (the hardware-validated version) — would gate a state no current contributor's machine reproduces; floating `latest` — an upstream release could break CI overnight |
| `esp-rs/xtensa-toolchain@v1.7.0` over raw `espup install`          | The official esp-rs action; espup's own README recommends it for CI, and it handles `GITHUB_TOKEN` wiring that avoids install-time API rate limits.                                                                                                                                            | `cargo install espup && espup install` — slower and rate-limit-prone                                                                                                         |
| Path filters, despite no existing workflow using them              | Keeps the ~2 min Xtensa install off unrelated PRs. `workflow_dispatch` on each workflow allows a forced run.                                                                                                                                                                                   | Match house style with no filters — every docs-only PR would pay the Xtensa install                                                                                          |

## Constraints

- **`avr-gcc` is required even for a `core`-only `no_std` build.** AVR 32-bit multiplication
  intrinsics come from `libgcc`, which `compiler-builtins` does not yet provide. `binutils-avr`
  supplies `avr-ld`; `avr-libc` the crt startup objects. All three are installed.
- **The Xtensa cache key must contain the version string.** `scripts/xtensa-toolchain.sh` hard-fails
  when it finds more than one version under `~/.rustup/toolchains/esp/xtensa-esp-elf/*`, which a
  version-less key could accumulate. That script is only sourced by `build-example.sh` /
  `run-example.sh`, so it does not affect these check-only jobs — but the constraint becomes live if
  a flash job is ever added.
- `~/.espressif` is deliberately **not** cached — unconfirmed whether it holds anything for a
  no_std build. Add only if an ESP-IDF job appears.
- The AVR nightly is pinned in two places that must move together: `avr_nightly` in the `justfile`
  and `toolchain:` in `cross-target-avr.yml`.

## Verified facts (settled empirically, 2026-08-12/13)

Recorded so they are not re-litigated:

- `RUSTUP_TOOLCHAIN: stable` (workflow house style) does **not** defeat `cargo +esp` /
  `cargo +nightly-…` — an explicit `+toolchain` takes precedence. Tested both ways.
- The workspace `.cargo/config.toml` sets `[unstable] build-std = ["std", "panic_abort"]`, but a
  command-line `-Z build-std=core` overrides it cleanly; `just check-avr-target` passes from the
  workspace root. (The config-file *merge* hazard is real, and is why
  `examples/avr-nano-rainbow/.cargo/config.toml` deliberately omits `build-std`.)
- `hal_dir` / `idf_dir` fall back to `target/hal` / `target/idf` on Linux with no RAM disk.
- espup installs `rust-src` unconditionally for the Xtensa toolchain, so `-Z build-std=core`
  needs no extra step.

Measured locally (Apple Silicon, warm cargo registry):

| Command                           | Time  | Output                                   |
|:----------------------------------|:------|:-----------------------------------------|
| `just check-avr-target`           | 9.5s  | builds `core` from source, checks driver |
| `just build-avr-example-all-bins` | 15.1s | 4 linked AVR ELF executables             |
| `just check-hal-c3`               | 13.7s | both invocations, examples included      |
| `just check-hal-xtensa`           | 13.2s | both invocations, after adding `pennant` |

## Open Questions

- [x] **The AVR example's git dependency is unpinned and its lockfile is untracked.**
      **Resolved 2026-08-13 by a three-agent research pass.** Decisive findings: `arduino-hal`,
      `avr-hal-generic` and `atmega-hal` all 404 on the crates.io API, so a git dependency is
      forced rather than chosen; upstream ships **no tags or releases**, so a commit SHA is the
      only pinnable ref; and the official `avr-hal-template` — the path the README tells every new
      user to start from — itself pins `rev = "e5c8f37fe484…"`, meaning our bare git dep was a
      *deviation from upstream's own documented practice*. `git ls-remote` confirmed that SHA is
      also avr-hal's current `main` HEAD, so the build was reproducible only by luck.
      Applied: pin that `rev` in the manifest, commit the lockfile via a `.gitignore` negation,
      and add `--locked` to `build-avr-example-all-bins`. These are complementary, not redundant —
      `rev` pins the git dep, the lockfile additionally pins the transitive crates.io set
      (`panic-halt`, `embedded-hal`, `rgb`, `avr-device`), and `--locked` fails loudly on
      manifest/lock drift.
      The early-warning signal that pinning would otherwise destroy is preserved by a separate
      weekly `cross-target-avr-upstream.yml`, which drops the pin and builds against upstream
      `main`. Cargo's own CI guidance recommends exactly this shape: commit the lockfile and test
      against latest dependencies via automation, rather than staying unpinned.
      Two things that turned out not to apply: `cargo-deny`'s `required-git-spec` cannot police
      this (the workspace lockfile has zero `arduino-hal` entries, so `just deny` never sees the
      excluded package), and avr-hal issue #282 (mismatched pins across family crates causing
      confusing trait errors) does not bite us — only one git crate is declared, and
      `avr-device 0.8.1` comes from crates.io.
- [ ] Should any of these become **required checks** under branch protection? Path-filtered
      workflows do not report a status when skipped, which interacts awkwardly with required checks
      on GitHub — worth confirming before enabling.

## State

- [x] Design approved
- [x] `check-hal-c3` recipe added (the only new recipe; `just --list` still ≤ 100 columns)
- [x] Three workflows authored: `cross-target-riscv.yml`, `cross-target-avr.yml`, `cross-target-xtensa.yml`
- [x] All six referenced recipes verified to resolve via `just --show`
- [x] All three workflow files verified to parse as YAML
- [x] Local verification of the new recipe and both AVR paths (see measurements above)
- [x] Review pass (`justfile-reviewer`, `build-engineer`) — two real defects found and fixed:
      `check-hal-xtensa` was missing `pennant`, and the two ESP path filters omitted `crates/ferriswheel/**`
- [x] **First CI run green** — 10/10 checks pass on the PR: `avr` 1m, `riscv` 59s, `xtensa` 2m,
      alongside the four pre-existing workflows and CodeQL (which also scanned the new workflow files)
- [x] Path filters confirmed to **fire** — all three jobs triggered on a PR that touched no
      `crates/**` files at all, via each workflow's self-gating `.github/workflows/<own-file>` entry
- [ ] Path filters confirmed to **exclude** — still unproven. No PR has yet touched one target's
      crates without the others, so the negative case (an AVR-only change skipping `xtensa`) has not
      been exercised. This is the design's main failure mode; watch the next crate-scoped PR
- [ ] Xtensa cache confirmed to restore — the 2m first run was necessarily cold, and cache save
      happens at job end, so a second run is needed to prove restore
- [x] Documentation updated — CHANGELOG entries added, roadmap item removed

## Session Log

- 2026-08-13 — Path filters missed `ferriswheel`: it is a dev-dependency of the esp-hal driver, used by 14 of its 17 examples, so `crates/ferriswheel/**` joined both ESP filters
- 2026-08-13 — The first green `xtensa` job is the first evidence that Xtensa 1.97.0.0 compiles the driver (compile-only; hardware validation ran on 1.95.0.0)
