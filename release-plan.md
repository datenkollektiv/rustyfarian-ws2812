# Release Plan

Release process for the `rustyfarian-ws2812` workspace.

> **Status:** Active process — last exercised for `v0.6.0` (published to crates.io 2026-05-20, all six crates).
> The staged publication flow below is proven; update this note after each release.

## Versioning

- **Scheme:** SemVer (0.x.y — pre-stable)
- **Model:** Single workspace version — all crates move to the same version together
- **Snapshot convention:** None; crates live at the released version between releases
- **Who decides version:** Maintainer; minor bump for new public API, patch for fixes only

## Branch and Tag Convention

- **Release branch:** `main`
- **Tag format:** `v0.X.Y` (e.g., `v0.2.0`)
- **Tagging:** Manual annotated tag on the release commit

## Pre-flight Checklist

Before any release:

- [ ] Build passes: `just check`
- [ ] Tests pass: `just test`
- [ ] `cargo deny` clean: `just deny`
- [ ] Pure-crate dry-run clean: `just release-dry-run` (runs `verify` plus `cargo publish --dry-run` for `bunting`, `pennant`, `ferriswheel`)
- [ ] Changelog has an `[Unreleased]` section with content for this release
- [ ] Version consistent across all crates: `crates/*/Cargo.toml`
- [ ] No uncommitted changes: `git status --short` is empty
- [ ] Working tree is on `main`
- [ ] Authenticated with crates.io (see Authentication below) — only for crates.io publishes

Note: driver crates cannot dry-run publish until the pure crates of the same version are live on crates.io — see Publish Order below.

## Version Bump

Files to update when bumping to `X.Y.Z`:

- `crates/bunting/Cargo.toml` — `version = "X.Y.Z"`
- `crates/ferriswheel/Cargo.toml` — `version = "X.Y.Z"`
- `crates/pennant/Cargo.toml` — `version = "X.Y.Z"`
- `crates/rustyfarian-esp-idf-ws2812/Cargo.toml` — `version = "X.Y.Z"`
- `crates/rustyfarian-esp-hal-ws2812/Cargo.toml` — `version = "X.Y.Z"`
- `crates/rustyfarian-avr-ws2812/Cargo.toml` — `version = "X.Y.Z"`

Post-release version bump: none (no snapshot pattern).

## Publish

**Target — pure-logic trio:** crates.io (`bunting`, `pennant`, `ferriswheel`) — published at `0.5.0`.
**Target — driver crates:** crates.io (`rustyfarian-avr-ws2812`, `rustyfarian-esp-idf-ws2812`, `rustyfarian-esp-hal-ws2812`) — first published at `0.6.0`.
**Credentials:** required for crates.io (see Authentication below).
**Downstream consumption:** versioned `[dependencies]` from crates.io for all six published crates.

### Authentication

`cargo publish` reads the API token from `~/.cargo/credentials.toml`.
Authenticate once per machine:

```sh
cargo login <your-crates-io-token>
```

The token comes from your crates.io account → Account Settings → API Tokens.
Alternatively, set `CARGO_REGISTRY_TOKEN` in the environment for the duration of the publish run:

```sh
CARGO_REGISTRY_TOKEN=<token> just release-publish bunting
```

### Crate Ownership

Sole owner of all six published crates today: `fwaibel@datenkollektiv.de`.

To add a co-owner (per crate):

```sh
cargo owner --add <github-username-or-team> bunting
cargo owner --add <github-username-or-team> pennant
cargo owner --add <github-username-or-team> ferriswheel
cargo owner --add <github-username-or-team> rustyfarian-avr-ws2812
cargo owner --add <github-username-or-team> rustyfarian-esp-idf-ws2812
cargo owner --add <github-username-or-team> rustyfarian-esp-hal-ws2812
```

Triggers for transition to a GitHub team owner (e.g. `github:datenkollektiv:wheel`): the first external contributor's PR merged, or at the `v1.0.0` cut.

### Publish Order

Publishing must be staged: driver crates declare `bunting`, `pennant`, and `ferriswheel` as workspace dependencies, so `cargo publish --dry-run` against them fails until the pure crates are live on crates.io.

**Stage 1 — pure-logic crates (any order, no cross-deps):**

```sh
just release-publish bunting
just release-publish pennant
just release-publish ferriswheel
```

Verify each listing on crates.io before continuing.

**Stage 2 — dry-run driver crates** (now that pure `0.X` is live):

The AVR driver builds against the host target; the ESP drivers require their own toolchain and target:

```sh
just release-dry-run-crate rustyfarian-avr-ws2812
just release-dry-run-hal
just release-dry-run-idf
```

**Stage 3 — publish driver crates (any order, no cross-deps between them):**

```sh
just release-publish rustyfarian-avr-ws2812
just release-publish-hal
just release-publish-idf
```

Verify each listing on crates.io: `https://crates.io/crates/<name>`.

Each `release-publish` / `release-dry-run-crate` call uses the host target and suits pure crates and the AVR driver.
`release-publish-hal` / `release-dry-run-hal` use `riscv32imac-unknown-none-elf` (requires `rustup target add riscv32imac-unknown-none-elf`).
`release-publish-idf` / `release-dry-run-idf` use `cargo +esp` with `riscv32imac-esp-espidf` (requires `espup`).

docs.rs will build `rustyfarian-esp-hal-ws2812` using the `[package.metadata.docs.rs]` target metadata.
`rustyfarian-esp-idf-ws2812` docs.rs builds will fail (docs.rs lacks the ESP-IDF build environment); this is expected for ESP-IDF crates — users are directed to the README for usage guidance.

## Changelog

**Location:** `CHANGELOG.md` (workspace-level — single source of truth for all crates, per the v1 decision in [`docs/features/crates-io-publication-v1.md`](docs/features/crates-io-publication-v1.md))
**Format:** [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
**Process:**

1. Replace `## [Unreleased]` with `## [X.Y.Z] - YYYY-MM-DD`
2. Add a fresh `## [Unreleased]` section above it (empty, for the next cycle)
3. Update the **Status** note at the top of this file (`last exercised for vX.Y.Z`, publish date, crate count) so the freshness marker doesn't drift

## Tagging

```sh
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin main --follow-tags
```

## GitHub Release

After pushing the tag, create a GitHub release:

- Navigate to the repository releases page
- Select tag `vX.Y.Z`
- Title: `v X.Y.Z`
- Body: paste the `## [X.Y.Z]` changelog section verbatim

## Rollback Procedure

### crates.io crates (v1 library trio)

`cargo yank` is the recourse for a published crate version:

```sh
cargo yank --version X.Y.Z bunting
```

A yanked version stays on crates.io (existing consumers' `Cargo.lock` files still resolve), but new dependents cannot pin to it.
Yanking is reversible:

```sh
cargo yank --version X.Y.Z --undo bunting
```

For pre-publish mistakes caught before any consumer locked on the version, yank the bad version, bump the patch version everywhere (for example `0.6.0` → `0.6.1`), and re-run `just release-dry-run` followed by `just release-publish`.

### Driver crates (crates.io — from 0.6.0 onwards)

Use `cargo yank` the same way as for the pure-logic crates:

```sh
cargo yank --version X.Y.Z rustyfarian-avr-ws2812
cargo yank --version X.Y.Z rustyfarian-esp-idf-ws2812
cargo yank --version X.Y.Z rustyfarian-esp-hal-ws2812
```

Then delete the tag and revert the version bump commit as above.

## Release Record Location

Each release produces files in `release/`:

1. `YYYY-MM-DD-<version>-preflight.md` — pre-flight assessment
2. `YYYY-MM-DD-<version>-plan.md` — ordered execution plan
3. `YYYY-MM-DD-<version>-record.md` — what was done and what remains
