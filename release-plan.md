# Release Plan

Release process for the `rustyfarian-ws2812` workspace.

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
- [ ] Crates.io dry-run clean: `just release-dry-run` (runs `verify` plus `cargo publish --dry-run` for each v1 crate)
- [ ] Changelog has an `[Unreleased]` section with content for this release
- [ ] Version consistent across all crates: `crates/*/Cargo.toml`
- [ ] No uncommitted changes: `git status --short` is empty
- [ ] Working tree is on `main`
- [ ] Authenticated with crates.io (see Authentication below) — only for crates.io publishes

## Version Bump

Files to update when bumping to `X.Y.Z`:

- `crates/bunting/Cargo.toml` — `version = "X.Y.Z"`
- `crates/ferriswheel/Cargo.toml` — `version = "X.Y.Z"`
- `crates/pennant/Cargo.toml` — `version = "X.Y.Z"`
- `crates/rustyfarian-esp-idf-ws2812/Cargo.toml` — `version = "X.Y.Z"`
- `crates/rustyfarian-esp-hal-ws2812/Cargo.toml` — `version = "X.Y.Z"`

Post-release version bump: none (no snapshot pattern).

## Publish

**Target — v1 library trio:** crates.io (`bunting`, `pennant`, `ferriswheel`).
**Target — driver crates:** git tag only (`rustyfarian-esp-idf-ws2812`, `rustyfarian-esp-hal-ws2812`, `rustyfarian-avr-ws2812` — not yet on crates.io; their first publish is a separate later wave).
**Credentials:** required for crates.io (see Authentication below).
**Downstream consumption:** versioned `[dependencies]` from crates.io for v1 library crates; git deps for driver crates until their own publish wave.

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

Sole owner of `bunting`, `pennant`, and `ferriswheel` today: `fwaibel@datenkollektiv.de`.

To add a co-owner (per crate):

```sh
cargo owner --add <github-username-or-team> bunting
cargo owner --add <github-username-or-team> pennant
cargo owner --add <github-username-or-team> ferriswheel
```

Triggers for transition to a GitHub team owner (e.g. `github:datenkollektiv:wheel`): the first external contributor's PR merged, or at the `v1.0.0` cut.

### Publish Order

The three v1 crates do not depend on each other, so any order works.
The canonical order is `bunting → pennant → ferriswheel`, recorded in [`docs/features/crates-io-publication-v1.md`](docs/features/crates-io-publication-v1.md).

Run each publish individually so you can verify the crates.io listing renders before moving on:

```sh
just release-publish bunting     # confirm Y/N at prompt; verify on https://crates.io/crates/bunting
just release-publish pennant     # confirm Y/N at prompt; verify on https://crates.io/crates/pennant
just release-publish ferriswheel # confirm Y/N at prompt; verify on https://crates.io/crates/ferriswheel
```

Each recipe runs `cargo publish -p <crate> --target <host>` after a `[confirm]` prompt.
The `--target` override is required because `.cargo/config.toml` defaults the workspace to an ESP cross-compile target.

## Changelog

**Location:** `CHANGELOG.md` (workspace-level — single source of truth for all crates, per the v1 decision in [`docs/features/crates-io-publication-v1.md`](docs/features/crates-io-publication-v1.md))
**Format:** [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
**Process:**

1. Replace `## [Unreleased]` with `## [X.Y.Z] - YYYY-MM-DD`
2. Add a fresh `## [Unreleased]` section above it (empty, for the next cycle)

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

For pre-publish mistakes caught before any consumer locked on the version, yank the bad version, bump (`X.Y.Z+1`) everywhere, and re-run `just release-dry-run` followed by `just release-publish`.

### Driver crates (git only)

If a mistake is caught after tagging but before any downstream picks it up:

1. Delete remote tag: `git push --delete origin vX.Y.Z`
2. Delete local tag: `git tag -d vX.Y.Z`
3. Revert the version bump commit: `git revert HEAD`
4. Push the revert: `git push origin main`

Since driver crates are not yet on crates.io, a yanked tag is sufficient — no registry rollback required.

## Release Record Location

Each release produces files in `release/`:

1. `YYYY-MM-DD-<version>-preflight.md` — pre-flight assessment
2. `YYYY-MM-DD-<version>-plan.md` — ordered execution plan
3. `YYYY-MM-DD-<version>-record.md` — what was done and what remains
